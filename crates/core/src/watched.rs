//! This session's folder-watch jobs, as History lists them. The shell keeps
//! the list, not the window, so a window built again after closing (plan 7)
//! still knows the clips the watch handled while it was gone.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::job::{JobEvent, JobId, JobResult, Origin};

/// How many watch jobs are kept, newest first.
pub const KEPT: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchedStatus {
    Queued,
    Running,
    Done,
    Failed,
    Canceled,
}

/// One clip the watch is compressing or has compressed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedJob {
    pub id: JobId,
    pub input: PathBuf,
    pub status: WatchedStatus,
    /// 0–100 for the current attempt.
    pub percent: f64,
    pub result: Option<JobResult>,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct WatchedJobs {
    jobs: Vec<WatchedJob>,
}

impl WatchedJobs {
    /// Follows one job event. Only `Queued` says where a job came from, so
    /// later events count only for jobs already listed.
    pub fn see(&mut self, event: &JobEvent) {
        if let JobEvent::Queued { id, input, origin: Origin::Watch } = event {
            self.jobs.insert(
                0,
                WatchedJob {
                    id: id.clone(),
                    input: input.clone(),
                    status: WatchedStatus::Queued,
                    percent: 0.0,
                    result: None,
                    error: None,
                },
            );
            self.jobs.truncate(KEPT);
            return;
        }
        let id = match event {
            JobEvent::Started { id, .. }
            | JobEvent::Progress { id, .. }
            | JobEvent::Done { id, .. }
            | JobEvent::Failed { id, .. }
            | JobEvent::Canceled { id } => id,
            _ => return,
        };
        let Some(job) = self.jobs.iter_mut().find(|job| &job.id == id) else { return };
        match event {
            JobEvent::Started { .. } => job.status = WatchedStatus::Running,
            JobEvent::Progress { percent, .. } => {
                job.status = WatchedStatus::Running;
                job.percent = *percent;
            }
            JobEvent::Done { result, .. } => {
                job.status = WatchedStatus::Done;
                job.result = Some(result.clone());
            }
            JobEvent::Failed { error, .. } => {
                job.status = WatchedStatus::Failed;
                job.error = Some(error.clone());
            }
            JobEvent::Canceled { .. } => job.status = WatchedStatus::Canceled,
            _ => {}
        }
    }

    /// The jobs, newest first.
    pub fn list(&self) -> Vec<WatchedJob> {
        self.jobs.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::Phase;

    fn queued(id: &str, origin: Origin) -> JobEvent {
        JobEvent::Queued { id: id.into(), input: PathBuf::from(format!("/clips/{id}.mp4")), origin }
    }

    fn result(id: &str) -> JobResult {
        JobResult {
            output: PathBuf::from(format!("/clips/{id}-20mb.mp4")),
            size_bytes: 9_000_000,
            original_bytes: 90_000_000,
            duration: 30.0,
            width: 1280,
            height: 720,
            video_kbps: 2400,
            audio_kbps: 128,
            encoder: "h264_nvenc".into(),
            hardware: true,
            fell_back_from: None,
            attempts: 1,
            elapsed_seconds: 4.0,
            target_mb: Some(20.0),
            replaced_original: false,
        }
    }

    fn progress(id: &str, percent: f64) -> JobEvent {
        JobEvent::Progress {
            id: id.into(),
            phase: Phase::Encoding,
            percent,
            speed: None,
            eta_seconds: None,
            out_bytes: 0,
        }
    }

    #[test]
    fn follows_watch_jobs_and_leaves_manual_ones_out() {
        let mut watched = WatchedJobs::default();
        watched.see(&queued("a", Origin::Watch));
        watched.see(&queued("m", Origin::Manual));
        watched.see(&progress("m", 50.0));
        watched.see(&queued("b", Origin::Watch));
        watched.see(&progress("a", 40.0));
        watched.see(&JobEvent::Done { id: "a".into(), result: result("a") });
        watched.see(&JobEvent::Failed { id: "b".into(), error: "The source can't be read.".into() });

        let list = watched.list();
        assert_eq!(list.iter().map(|j| j.id.as_str()).collect::<Vec<_>>(), ["b", "a"], "newest first");
        assert_eq!(list[0].status, WatchedStatus::Failed);
        assert_eq!(list[0].error.as_deref(), Some("The source can't be read."));
        assert_eq!(list[1].status, WatchedStatus::Done);
        assert_eq!(list[1].percent, 40.0);
        assert_eq!(list[1].result.as_ref().map(|r| r.output.clone()), Some(PathBuf::from("/clips/a-20mb.mp4")));
    }

    #[test]
    fn keeps_the_newest_few() {
        let mut watched = WatchedJobs::default();
        for i in 0..KEPT + 5 {
            watched.see(&queued(&i.to_string(), Origin::Watch));
        }
        let list = watched.list();
        assert_eq!(list.len(), KEPT);
        assert_eq!(list[0].id, (KEPT + 4).to_string());
        watched.see(&JobEvent::Canceled { id: "0".into() });
        assert!(watched.list().iter().all(|j| j.status == WatchedStatus::Queued), "a job no longer listed is ignored");
    }

    #[test]
    fn serializes_as_the_window_reads_it() {
        let mut watched = WatchedJobs::default();
        watched.see(&queued("a", Origin::Watch));
        let json = serde_json::to_value(watched.list()).unwrap();
        assert_eq!(json[0]["status"], "queued");
        assert_eq!(json[0]["input"], "/clips/a.mp4");
        assert!(json[0]["result"].is_null() && json[0]["error"].is_null() && json[0]["percent"] == 0.0);
    }
}
