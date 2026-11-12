//! The job queue: runs FFmpeg, reports progress, and makes sure the result
//! fits.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::{mpsc, oneshot, watch, Notify};

use crate::command::{self, EncodeArgs, Pass};
use crate::encoders::{self, Encoder};
use crate::error::{Error, IoContext, Result};
use crate::ffmpeg::Tools;
use crate::hardware::{self, Report};
use crate::history::{self, Entry, History};
use crate::job::{EncodeOptions, JobEvent, JobId, JobResult, JobSpec, Phase};
use crate::output;
use crate::plan::{self, Plan};
use crate::probe::{self, MediaInfo};
use crate::progress::{Eta, Reader};
use crate::settings::SettingsStore;

/// How many times a too-large result is encoded again.
const SIZE_RETRIES: u32 = 2;

/// Receives everything the engine has to say. The desktop shell forwards
/// these to the window; tests collect them.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: JobEvent);
}

impl<F: Fn(JobEvent) + Send + Sync + 'static> EventSink for F {
    fn emit(&self, event: JobEvent) {
        self(event)
    }
}

/// Lets a fixed number of jobs run at once, first come first served.
struct Gate {
    state: Mutex<GateState>,
    changed: Notify,
}

#[derive(Default)]
struct GateState {
    running: usize,
    waiting: VecDeque<u64>,
    next_ticket: u64,
}

struct Permit<'a>(&'a Gate);

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.0.state.lock().expect("gate lock").running -= 1;
        self.0.changed.notify_waiters();
    }
}

impl Gate {
    /// Takes a place in line. Called when a job is submitted, so jobs run in
    /// the order they were handed over rather than the order their tasks
    /// happen to be scheduled.
    fn line_up(&self) -> u64 {
        let mut state = self.state.lock().expect("gate lock");
        state.next_ticket += 1;
        let ticket = state.next_ticket;
        state.waiting.push_back(ticket);
        ticket
    }

    async fn enter(
        &self,
        ticket: u64,
        limit: impl Fn() -> usize,
        cancel: &mut watch::Receiver<bool>,
    ) -> Result<Permit<'_>> {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            // Register before checking, so a release in between is not missed.
            changed.as_mut().enable();
            {
                let mut state = self.state.lock().expect("gate lock");
                if state.waiting.front() == Some(&ticket) && state.running < limit().max(1) {
                    state.waiting.pop_front();
                    state.running += 1;
                    drop(state);
                    // Let the next in line check too, in case there is room.
                    self.changed.notify_waiters();
                    return Ok(Permit(self));
                }
            }
            tokio::select! {
                _ = &mut changed => {}
                _ = canceled(cancel) => {
                    self.state.lock().expect("gate lock").waiting.retain(|t| *t != ticket);
                    self.changed.notify_waiters();
                    return Err(Error::Canceled);
                }
            }
        }
    }
}

/// Resolves once the job has been canceled.
async fn canceled(cancel: &mut watch::Receiver<bool>) {
    loop {
        if *cancel.borrow() {
            return;
        }
        if cancel.changed().await.is_err() {
            // The sender is gone; nobody can cancel any more.
            std::future::pending::<()>().await;
        }
    }
}

type Waiter = oneshot::Sender<std::result::Result<JobResult, String>>;

struct Inner {
    settings: Arc<SettingsStore>,
    history: Arc<History>,
    sink: Arc<dyn EventSink>,
    bundled: Vec<PathBuf>,
    work_dir: PathBuf,
    tools: RwLock<Option<Tools>>,
    report: tokio::sync::Mutex<Option<Arc<Report>>>,
    jobs: Mutex<HashMap<JobId, watch::Sender<bool>>>,
    gate: Gate,
}

/// The engine. Cheap to clone; all clones share one queue.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<Inner>,
}

impl Engine {
    /// * `data_dir` holds `settings.json` and `history.json`.
    /// * `work_dir` is scratch space for two-pass statistics.
    /// * `bundled` lists folders that may hold an FFmpeg shipped with the app.
    pub fn new(data_dir: &Path, work_dir: &Path, bundled: Vec<PathBuf>, sink: Arc<dyn EventSink>) -> Engine {
        let settings = Arc::new(SettingsStore::open(data_dir.join("settings.json")));
        let history = Arc::new(History::open(data_dir.join("history.json")));
        let _ = history.prune(settings.get().history_keep_days);
        Engine {
            inner: Arc::new(Inner {
                settings,
                history,
                sink,
                bundled,
                work_dir: work_dir.to_path_buf(),
                tools: RwLock::new(None),
                report: tokio::sync::Mutex::new(None),
                jobs: Mutex::new(HashMap::new()),
                gate: Gate { state: Mutex::new(GateState::default()), changed: Notify::new() },
            }),
        }
    }

    pub fn settings(&self) -> &Arc<SettingsStore> {
        &self.inner.settings
    }

    pub fn history(&self) -> &Arc<History> {
        &self.inner.history
    }

    /// Renames a finished file to what was typed, keeping its extension,
    /// and points its history entry at the new name.
    pub fn rename_result(&self, path: &Path, typed: &str) -> Result<PathBuf> {
        let renamed = output::rename(path, typed)?;
        if let Err(e) = self.inner.history.retarget(path, &renamed) {
            log::warn!("could not update history: {e}");
        }
        Ok(renamed)
    }

    /// FFmpeg and FFprobe, located once and remembered.
    pub fn tools(&self) -> Result<Tools> {
        if let Some(tools) = self.inner.tools.read().expect("tools lock").clone() {
            return Ok(tools);
        }
        let folder = self.inner.settings.get().ffmpeg_folder;
        let tools = Tools::locate(folder.as_deref(), &self.inner.bundled)?;
        *self.inner.tools.write().expect("tools lock") = Some(tools.clone());
        Ok(tools)
    }

    /// Forgets the located FFmpeg and the encoder tests. Call after the
    /// FFmpeg folder setting changes.
    pub async fn forget_tools(&self) {
        *self.inner.tools.write().expect("tools lock") = None;
        *self.inner.report.lock().await = None;
    }

    /// Wakes queued jobs so they notice a changed parallel-jobs setting.
    pub fn settings_changed(&self) {
        self.inner.gate.changed.notify_waiters();
    }

    /// Which encoders work here. The first call runs the tests; later calls
    /// return the remembered answer unless `refresh` is set.
    pub async fn hardware(&self, refresh: bool) -> Result<Arc<Report>> {
        let mut slot = self.inner.report.lock().await;
        if let (Some(report), false) = (slot.as_ref(), refresh) {
            return Ok(report.clone());
        }
        let tools = self.tools()?;
        let report = Arc::new(hardware::detect(&tools).await);
        *slot = Some(report.clone());
        Ok(report)
    }

    pub async fn probe(&self, path: &Path) -> Result<MediaInfo> {
        probe::probe(&self.tools()?, path).await
    }

    /// Number of jobs queued or running.
    pub fn active(&self) -> usize {
        self.inner.jobs.lock().expect("jobs lock").len()
    }

    /// Adds a job to the queue and returns its id straight away.
    pub fn submit(&self, spec: JobSpec) -> JobId {
        self.start(spec, None)
    }

    /// Adds a job and also returns a receiver for its outcome.
    pub fn submit_and_wait(&self, spec: JobSpec) -> (JobId, oneshot::Receiver<std::result::Result<JobResult, String>>) {
        let (tx, rx) = oneshot::channel();
        (self.start(spec, Some(tx)), rx)
    }

    fn start(&self, spec: JobSpec, waiter: Option<Waiter>) -> JobId {
        let id: JobId = uuid::Uuid::new_v4().simple().to_string();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        self.inner.jobs.lock().expect("jobs lock").insert(id.clone(), cancel_tx);
        self.inner.sink.emit(JobEvent::Queued { id: id.clone(), input: spec.input.clone(), origin: spec.origin });

        let ticket = self.inner.gate.line_up();
        let engine = self.clone();
        let job_id = id.clone();
        tokio::spawn(async move {
            let outcome = engine.run(&job_id, &spec, ticket, cancel_rx).await;
            engine.inner.jobs.lock().expect("jobs lock").remove(&job_id);
            let reply = match outcome {
                Ok(result) => {
                    engine.record(&job_id, &spec, &result);
                    engine.inner.sink.emit(JobEvent::Done { id: job_id, result: result.clone() });
                    Ok(result)
                }
                Err(Error::Canceled) => {
                    engine.inner.sink.emit(JobEvent::Canceled { id: job_id });
                    Err("canceled".to_string())
                }
                Err(error) => {
                    let message = error.to_string();
                    engine.inner.sink.emit(JobEvent::Failed { id: job_id, error: message.clone() });
                    Err(message)
                }
            };
            if let Some(waiter) = waiter {
                let _ = waiter.send(reply);
            }
        });
        id
    }

    /// Stops a job, queued or running. Its partial output is removed.
    pub fn cancel(&self, id: &str) -> bool {
        match self.inner.jobs.lock().expect("jobs lock").get(id) {
            Some(cancel) => cancel.send(true).is_ok(),
            None => false,
        }
    }

    pub fn cancel_all(&self) {
        for cancel in self.inner.jobs.lock().expect("jobs lock").values() {
            let _ = cancel.send(true);
        }
    }

    fn record(&self, id: &str, spec: &JobSpec, result: &JobResult) {
        let settings = self.inner.settings.get();
        if !settings.history_enabled {
            return;
        }
        let entry = Entry {
            id: id.to_string(),
            finished_at: history::now(),
            input: spec.input.clone(),
            trimmed: spec.trim.is_some(),
            origin: spec.origin,
            result: result.clone(),
        };
        if let Err(e) = self.inner.history.add(entry) {
            log::warn!("could not save history: {e}");
        }
    }

    fn parallel_limit(&self) -> usize {
        match self.inner.settings.get().parallel_jobs {
            0 => {
                let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
                (cores / 8).clamp(1, 3)
            }
            n => n as usize,
        }
    }

    fn log(&self, id: &str, line: impl Into<String>) {
        self.inner.sink.emit(JobEvent::Log { id: id.to_string(), line: line.into() });
    }

    async fn run(&self, id: &str, spec: &JobSpec, ticket: u64, mut cancel: watch::Receiver<bool>) -> Result<JobResult> {
        let _permit = self.inner.gate.enter(ticket, || self.parallel_limit(), &mut cancel).await?;
        let started = Instant::now();
        let sink = &self.inner.sink;
        let tools = self.tools()?;

        sink.emit(JobEvent::Progress {
            id: id.to_string(),
            phase: Phase::Analyzing,
            percent: 0.0,
            speed: None,
            eta_seconds: None,
            out_bytes: 0,
        });
        let input = absolute(&spec.input)?;
        let info = probe::probe(&tools, &input).await?;
        let plan = plan::plan(&info, spec.trim, &spec.options)?;
        for note in &plan.notes {
            self.log(id, note.clone());
        }

        // Pick the encoder.
        let report = self.hardware(false).await?;
        let disabled: HashSet<String> = self.inner.settings.get().disabled_encoders.into_iter().collect();
        let (mut encoder, mut fell_back_from) = if spec.options.audio_only {
            (encoders::find("libx264").expect("catalog"), None)
        } else {
            let choice = choose_encoder(&report, &spec.options, &disabled)?;
            if let Some((from, reason)) = &choice.1 {
                sink.emit(JobEvent::Fallback {
                    id: id.to_string(),
                    from: from.clone(),
                    to: choice.0.name.to_string(),
                    reason: reason.clone(),
                });
            }
            (choice.0, choice.1.map(|(from, _)| from))
        };

        // Where things go.
        let extension = command::extension(&plan, &spec.options);
        let out_dir = match &spec.output_dir {
            Some(dir) => absolute(dir)?,
            None => input.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(".")),
        };
        std::fs::create_dir_all(&out_dir).ctx(format!("could not create {}", out_dir.display()))?;
        let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "video".into());
        let partial = out_dir.join(format!(".{stem}.{}.part.{extension}", &id[..8]));
        let scratch = self.inner.work_dir.join(id);
        std::fs::create_dir_all(&scratch).ctx("could not create a scratch folder")?;
        let _cleanup = Cleanup { partial: partial.clone(), scratch: scratch.clone() };

        sink.emit(JobEvent::Started {
            id: id.to_string(),
            encoder: if spec.options.audio_only {
                plan.audio.map(|a| a.encoder()).unwrap_or("aac").to_string()
            } else {
                encoder.name.to_string()
            },
            hardware: !spec.options.audio_only && encoder.family.is_hardware(),
            video_kbps: plan.video_kbps,
            width: plan.width,
            height: plan.height,
        });

        let mut video_kbps = plan.video_kbps;
        let mut hardware_decode = spec.options.hardware_decode;
        let mut attempts = 0u32;
        let mut size_retries = 0u32;
        let mut previous_size: Option<u64> = None;
        let size = loop {
            attempts += 1;
            let passes: &[Pass] = if !spec.options.audio_only && command::uses_two_passes(encoder, &spec.options) {
                &[Pass::First, Pass::Second]
            } else {
                &[Pass::Single]
            };
            let mut failure = None;
            for (index, pass) in passes.iter().enumerate() {
                let args = command::build(&EncodeArgs {
                    input: &input,
                    output: &partial,
                    info: &info,
                    plan: &plan,
                    options: &spec.options,
                    encoder,
                    device: report.device(encoder.name),
                    video_kbps,
                    hardware_decode,
                    pass: *pass,
                });
                self.log(id, command::display(&tools.ffmpeg, &args));
                let span = Span {
                    phase: if *pass == Pass::First { Phase::Analysis } else { Phase::Encoding },
                    from: index as f64 / passes.len() as f64 * 100.0,
                    to: (index + 1) as f64 / passes.len() as f64 * 100.0,
                };
                match self.run_ffmpeg(id, &tools, args, &scratch, &plan, span, &mut cancel).await {
                    Ok(()) => {}
                    Err(Error::Canceled) => return Err(Error::Canceled),
                    Err(e) => {
                        failure = Some(e);
                        break;
                    }
                }
            }

            if let Some(error) = failure {
                attempts -= 1;
                let _ = std::fs::remove_file(&partial);
                if hardware_decode {
                    hardware_decode = false;
                    self.log(id, format!("Hardware decoding failed ({error}). Decoding on the CPU instead."));
                    continue;
                }
                let cpu = report.cpu(encoder.codec).filter(|cpu| cpu.name != encoder.name);
                match cpu {
                    Some(cpu) if encoder.family.is_hardware() && !spec.options.audio_only => {
                        sink.emit(JobEvent::Fallback {
                            id: id.to_string(),
                            from: encoder.name.to_string(),
                            to: cpu.name.to_string(),
                            reason: error.to_string(),
                        });
                        fell_back_from.get_or_insert_with(|| encoder.name.to_string());
                        encoder = cpu;
                        continue;
                    }
                    _ => return Err(error),
                }
            }

            let size = std::fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);
            if size == 0 {
                return Err(Error::Encode("FFmpeg finished but wrote no output".into()));
            }
            let Some(target) = plan.target_bytes else { break size };
            if size <= target || spec.options.audio_only {
                break size;
            }
            let over_percent = (size as f64 / target as f64 - 1.0) * 100.0;
            let _ = std::fs::remove_file(&partial);
            let attempt = Attempt {
                hardware: encoder.family.is_hardware(),
                size_retries,
                video_kbps,
                size,
                previous_size,
                target,
                duration: plan.duration,
            };
            match when_over(&attempt) {
                Over::Retry(lower) => {
                    size_retries += 1;
                    video_kbps = lower;
                    previous_size = Some(size);
                    sink.emit(JobEvent::Retry { id: id.to_string(), attempt: size_retries, over_percent, video_kbps });
                }
                Over::UseCpu => {
                    // Hardware encoders have a bitrate floor (h264_nvenc will
                    // not go much under 600 kbps at 720p30) and drift over
                    // their target; the CPU encoders hit it.
                    let Some(cpu) = report.cpu(encoder.codec).filter(|cpu| cpu.name != encoder.name) else {
                        return Err(over_limit(over_percent, attempts));
                    };
                    let reason = format!("{} could not get under the limit ({over_percent:.1}% over)", encoder.name);
                    self.log(id, format!("{reason}. Encoding on the CPU instead."));
                    sink.emit(JobEvent::Fallback {
                        id: id.to_string(),
                        from: encoder.name.to_string(),
                        to: cpu.name.to_string(),
                        reason,
                    });
                    fell_back_from.get_or_insert_with(|| encoder.name.to_string());
                    encoder = cpu;
                    video_kbps = plan.video_kbps;
                    size_retries = 0;
                    previous_size = None;
                }
                Over::GiveUp => return Err(over_limit(over_percent, attempts)),
            }
        };

        // Publish under the final name.
        sink.emit(JobEvent::Progress {
            id: id.to_string(),
            phase: Phase::Finalizing,
            percent: 100.0,
            speed: None,
            eta_seconds: Some(0.0),
            out_bytes: size,
        });
        // A typed name wins; replacing keeps the original's name; else name-20mb.
        let default_stem = if spec.replace_original { stem.clone() } else { output_stem(&stem, &plan, &spec.options) };
        let final_stem =
            spec.output_name.as_deref().and_then(|typed| output::clean_name(typed, extension)).unwrap_or(default_stem);
        let original = spec.replace_original.then_some(input.as_path());
        let placed = output::place(&partial, &out_dir, &final_stem, extension, original, |file| {
            trash::delete(file).map_err(std::io::Error::other)
        })?;
        if let Some(note) = &placed.note {
            self.log(id, note.clone());
        }
        let output = placed.path;

        Ok(JobResult {
            output,
            size_bytes: size,
            original_bytes: info.size_bytes,
            duration: plan.duration,
            width: plan.width,
            height: plan.height,
            video_kbps,
            audio_kbps: plan.audio_kbps,
            encoder: if spec.options.audio_only {
                plan.audio.map(|a| a.encoder()).unwrap_or("aac").to_string()
            } else {
                encoder.name.to_string()
            },
            hardware: !spec.options.audio_only && encoder.family.is_hardware(),
            fell_back_from,
            attempts,
            elapsed_seconds: started.elapsed().as_secs_f64(),
            target_mb: plan.target_bytes.map(|_| spec.options.target_mb),
            replaced_original: placed.replaced,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_ffmpeg(
        &self,
        id: &str,
        tools: &Tools,
        args: Vec<std::ffi::OsString>,
        cwd: &Path,
        plan: &Plan,
        span: Span,
        cancel: &mut watch::Receiver<bool>,
    ) -> Result<()> {
        let mut cmd = tools.ffmpeg();
        cmd.args(&args).current_dir(cwd).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| Error::io("could not start FFmpeg", e))?;
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");

        let (tx, mut rx) = mpsc::channel::<(bool, String)>(256);
        tokio::spawn(forward_lines(stdout, false, tx.clone()));
        tokio::spawn(forward_lines(stderr, true, tx));

        let mut reader = Reader::default();
        let mut eta = Eta::default();
        let mut recent: VecDeque<String> = VecDeque::new();
        loop {
            tokio::select! {
                line = rx.recv() => match line {
                    Some((false, line)) => {
                        let Some(tick) = reader.line(&line) else { continue };
                        let fraction = (tick.out_time / plan.duration).clamp(0.0, 1.0);
                        let remaining = (plan.duration - tick.out_time).max(0.0);
                        self.inner.sink.emit(JobEvent::Progress {
                            id: id.to_string(),
                            phase: span.phase,
                            // Leave the last sliver for the container finalize.
                            percent: (span.from + (span.to - span.from) * fraction).min(99.5),
                            speed: tick.speed,
                            eta_seconds: eta.update(tick.speed, remaining),
                            out_bytes: tick.total_size,
                        });
                    }
                    Some((true, line)) => {
                        if recent.len() == 12 {
                            recent.pop_front();
                        }
                        recent.push_back(line.clone());
                        self.log(id, line);
                    }
                    None => break,
                },
                _ = canceled(cancel) => {
                    let _ = child.kill().await;
                    return Err(Error::Canceled);
                }
            }
        }
        let status = tokio::select! {
            status = child.wait() => status.map_err(|e| Error::io("FFmpeg did not finish", e))?,
            _ = canceled(cancel) => {
                let _ = child.kill().await;
                return Err(Error::Canceled);
            }
        };
        if status.success() {
            Ok(())
        } else {
            let lines: Vec<String> = recent.into_iter().collect();
            Err(Error::Encode(explain(&lines)))
        }
    }
}

/// The slice of the progress bar one FFmpeg run fills.
#[derive(Clone, Copy)]
struct Span {
    phase: Phase,
    from: f64,
    to: f64,
}

/// Removes the partial output and scratch folder however the job ends.
struct Cleanup {
    partial: PathBuf,
    scratch: PathBuf,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        patiently(|| std::fs::remove_file(&self.partial));
        patiently(|| std::fs::remove_dir_all(&self.scratch));
    }
}

/// Runs a removal, trying again for about five seconds while the file is in
/// use, backing off from 25 ms to 400 ms between tries. On Windows a killed
/// FFmpeg's output can stay locked for a moment after the process exits
/// (antivirus and the indexer open new files), and removal then fails with a
/// sharing violation. On a CI runner Defender held one for over a second.
fn patiently(remove: impl Fn() -> std::io::Result<()>) {
    let mut pause = 25;
    let mut waited = 0;
    loop {
        match remove() {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound && waited < 5_000 => {
                std::thread::sleep(std::time::Duration::from_millis(pause));
                waited += pause;
                pause = (pause * 2).min(400);
            }
            _ => return,
        }
    }
}

/// Reads a pipe line by line, tolerating bytes that are not UTF-8.
async fn forward_lines(pipe: impl AsyncRead + Unpin, is_log: bool, tx: mpsc::Sender<(bool, String)>) {
    let mut reader = BufReader::new(pipe);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buffer);
                // FFmpeg redraws some lines with carriage returns.
                for piece in text.split(['\r', '\n']) {
                    let piece = piece.trim_end();
                    if !piece.is_empty() && tx.send((is_log, piece.to_string())).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}

/// Picks the most telling line from FFmpeg's last words.
fn explain(lines: &[String]) -> String {
    // FFmpeg always ends with this; the cause is a line or two above.
    let useful: Vec<&String> = lines.iter().filter(|l| !l.contains("Conversion failed")).collect();
    let telling = useful.iter().rev().find(|line| {
        let lower = line.to_lowercase();
        ["error", "invalid", "failed", "cannot", "could not", "no such", "not found", "unsupported", "denied"]
            .iter()
            .any(|word| lower.contains(word))
    });
    match telling.or(useful.last()) {
        Some(line) => line.trim().to_string(),
        None => "no details were given".to_string(),
    }
}

fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path).ctx(format!("could not resolve {}", path.display()))
}

/// The encoder for a job, and what it replaced if the requested one is out.
fn choose_encoder(
    report: &Report,
    options: &EncodeOptions,
    disabled: &HashSet<String>,
) -> Result<(Encoder, Option<(String, String)>)> {
    if let Some(name) = &options.encoder {
        let wanted =
            encoders::find(name).ok_or_else(|| Error::Invalid(format!("{name} is not an encoder this app knows")))?;
        if report.works(name) {
            return Ok((wanted, None));
        }
        let reason = report
            .status(name)
            .and_then(|s| s.detail.clone())
            .unwrap_or_else(|| "it is not available on this machine".into());
        return match report.cpu(wanted.codec).filter(|cpu| cpu.name != wanted.name) {
            Some(cpu) => Ok((cpu, Some((name.clone(), reason)))),
            None => Err(Error::Invalid(format!(
                "{} does not work on this machine ({reason}) and there is no CPU encoder to fall back on",
                encoders::describe(name)
            ))),
        };
    }
    report.best(options.codec, disabled).or_else(|| report.cpu(options.codec)).map(|e| (e, None)).ok_or_else(|| {
        Error::Invalid(format!(
            "No {} encoder works on this machine. This FFmpeg build may not include one.",
            options.codec.label()
        ))
    })
}

/// An encode that came out over the size limit.
struct Attempt {
    hardware: bool,
    /// Lower-bitrate retries already made on this encoder.
    size_retries: u32,
    video_kbps: u32,
    size: u64,
    /// The size of the previous try on this encoder, if this was a retry.
    previous_size: Option<u64>,
    target: u64,
    duration: f64,
}

#[derive(Debug, PartialEq)]
enum Over {
    /// Encode again at this video bitrate.
    Retry(u32),
    /// Start again on the CPU encoder for the same codec.
    UseCpu,
    /// Stop without saving anything.
    GiveUp,
}

/// What to do when a result is over the limit. The limit is strict: an
/// over-limit file is never saved, so this ends in a retry, a switch to
/// the CPU, or an error.
fn when_over(a: &Attempt) -> Over {
    let out_of_road = if a.hardware { Over::UseCpu } else { Over::GiveUp };
    // A lower bitrate that barely shrank the file means the encoder is at
    // its floor; more retries would only waste time.
    if a.hardware && a.previous_size.is_some_and(|previous| a.size as f64 >= previous as f64 * 0.95) {
        return Over::UseCpu;
    }
    if a.size_retries >= SIZE_RETRIES {
        return out_of_road;
    }
    let lower = lower_bitrate(a.video_kbps, a.size, a.target, a.duration);
    if lower >= a.video_kbps {
        return out_of_road;
    }
    Over::Retry(lower)
}

fn over_limit(over_percent: f64, attempts: u32) -> Error {
    Error::OverLimit(format!(
        "Could not get under the size limit: still {over_percent:.1}% over after {attempts} tries, so nothing was saved. \
         Pick a larger size, keep a shorter part, or choose a lower resolution."
    ))
}

/// A lower video bitrate after a result came out `size` bytes against a
/// `target`: the more careful of "scale by the miss" and "subtract the miss".
fn lower_bitrate(video_kbps: u32, size: u64, target: u64, duration: f64) -> u32 {
    let ratio = target as f64 / size as f64;
    let proportional = f64::from(video_kbps) * ratio * 0.97;
    let over_kbps = (size - target) as f64 * 8.0 / 1000.0 / duration.max(0.1);
    let subtractive = f64::from(video_kbps) - over_kbps * 1.25;
    // When the overshoot dwarfs the video bitrate the subtraction is
    // meaningless; scale instead.
    let lower = if subtractive > proportional * 0.5 { proportional.min(subtractive) } else { proportional };
    (lower.floor().max(16.0)) as u32
}

/// `clip` → `clip-10mb`, `clip-2500k` or `clip-audio`.
fn output_stem(stem: &str, plan: &Plan, options: &EncodeOptions) -> String {
    if options.audio_only {
        format!("{stem}-audio")
    } else if plan.target_bytes.is_some() {
        format!("{stem}-{}mb", plan::trim_number(options.target_mb))
    } else {
        format!("{stem}-{}k", plan.video_kbps)
    }
}

/// True for names this app produced, so the folder watch leaves them alone.
pub fn looks_generated(stem: &str) -> bool {
    // Strip a numeric "-2" style suffix added to avoid a name clash.
    let base = match stem.rsplit_once('-') {
        Some((head, tail)) if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) => head,
        _ => stem,
    };
    let Some((_, tail)) = base.rsplit_once('-') else { return false };
    if tail == "audio" {
        return true;
    }
    let number = |text: &str| !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || c == '.');
    tail.strip_suffix("mb").is_some_and(number) || tail.strip_suffix('k').is_some_and(number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_names() {
        for name in ["clip-10mb", "clip-9.7mb", "clip-10mb-2", "clip-2500k", "clip-audio", "a-b-8mb-13"] {
            assert!(looks_generated(name), "{name}");
        }
        for name in ["clip", "clip-final", "10mb", "clip-mb", "holiday-2", "clip-10mbps", "audio"] {
            assert!(!looks_generated(name), "{name}");
        }
    }

    #[test]
    fn free_names_do_not_collide() {
        let dir = tempfile::tempdir().unwrap();
        let first = output::free_name(dir.path(), "clip-10mb", "mp4");
        assert_eq!(first.file_name().unwrap(), "clip-10mb.mp4");
        std::fs::write(&first, b"x").unwrap();
        let second = output::free_name(dir.path(), "clip-10mb", "mp4");
        assert_eq!(second.file_name().unwrap(), "clip-10mb-2.mp4");
    }

    #[test]
    fn retry_bitrate_always_drops_enough() {
        // 10% over on a 60 s clip at 1300 kbps.
        let lower = lower_bitrate(1300, 11_000_000, 10_000_000, 60.0);
        assert!(lower < 1300 * 10 / 11, "{lower}");
        // A big overshoot scales proportionally.
        let lower = lower_bitrate(2000, 25_000_000, 10_000_000, 60.0);
        assert!((16..=800).contains(&lower), "{lower}");
        // Never below the floor.
        assert_eq!(lower_bitrate(20, 2_000_000, 100_000, 5.0), 16);
    }

    fn over(hardware: bool, size_retries: u32, size: u64, previous_size: Option<u64>) -> Over {
        when_over(&attempt(hardware, size_retries, size, previous_size))
    }

    #[test]
    fn a_small_miss_is_retried_lower() {
        assert!(matches!(over(false, 0, 10_500_000, None), Over::Retry(kbps) if kbps < 1000));
        assert!(matches!(over(true, 0, 10_500_000, None), Over::Retry(kbps) if kbps < 1000));
    }

    #[test]
    fn hardware_at_its_floor_moves_to_the_cpu_at_once() {
        // The retry asked for less but the file barely shrank (h264_nvenc on
        // a busy 720p clip at ~300 kbps).
        assert_eq!(over(true, 1, 18_500_000, Some(18_700_000)), Over::UseCpu);
        // A retry that did shrink the file gets another go.
        assert!(matches!(over(true, 1, 10_400_000, Some(12_000_000)), Over::Retry(_)));
    }

    #[test]
    fn out_of_retries_never_keeps_the_file() {
        assert_eq!(over(true, SIZE_RETRIES, 10_100_000, Some(11_000_000)), Over::UseCpu);
        assert_eq!(over(false, SIZE_RETRIES, 10_100_000, Some(11_000_000)), Over::GiveUp);
    }

    #[test]
    fn a_bitrate_that_cannot_drop_ends_the_tries() {
        let at_floor = |hardware| when_over(&Attempt { video_kbps: 16, ..attempt(hardware, 0, 10_500_000, None) });
        assert_eq!(at_floor(false), Over::GiveUp);
        assert_eq!(at_floor(true), Over::UseCpu);
    }

    fn attempt(hardware: bool, size_retries: u32, size: u64, previous_size: Option<u64>) -> Attempt {
        Attempt { hardware, size_retries, video_kbps: 1000, size, previous_size, target: 10_000_000, duration: 60.0 }
    }

    #[test]
    fn explains_with_the_telling_line() {
        let lines = vec![
            "Stream mapping:".to_string(),
            "[h264_nvenc @ 0x1] Cannot load libcuda.so.1".to_string(),
            "Conversion failed!".to_string(),
        ];
        assert_eq!(explain(&lines), "[h264_nvenc @ 0x1] Cannot load libcuda.so.1");
        assert_eq!(explain(&[]), "no details were given");
    }
}
