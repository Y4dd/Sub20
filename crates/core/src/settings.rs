//! Settings and saved profiles, kept in one JSON file.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::encoders::Codec;
use crate::error::{Error, IoContext, Result};
use crate::job::{Container, Effort, EncodeOptions};

/// A named set of options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub options: EncodeOptions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum OutputMode {
    /// Next to the original.
    #[default]
    SameFolder,
    /// In one chosen folder.
    Folder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Originals {
    #[default]
    Keep,
    Delete,
    /// Move into a `processed` folder beside the result.
    Move,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Existing {
    /// Only compress files that arrive after the watch starts.
    #[default]
    NewOnly,
    /// Also compress what is already in the folder.
    ProcessExisting,
}

/// Compress whatever lands in a folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WatchSettings {
    pub enabled: bool,
    pub folder: Option<PathBuf>,
    /// Profile to use. `None` uses the default profile.
    pub profile: Option<String>,
    pub output_mode: OutputMode,
    pub output_folder: Option<PathBuf>,
    pub originals: Originals,
    pub existing: Existing,
    pub recursive: bool,
    /// A file must stop changing for this long before it is picked up.
    pub stable_seconds: u32,
    /// How often the folder is scanned.
    pub poll_seconds: u32,
}

impl Default for WatchSettings {
    fn default() -> Self {
        WatchSettings {
            enabled: false,
            folder: None,
            profile: None,
            output_mode: OutputMode::SameFolder,
            output_folder: None,
            originals: Originals::Keep,
            existing: Existing::NewOnly,
            recursive: false,
            stable_seconds: 5,
            poll_seconds: 5,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub profiles: Vec<Profile>,
    /// Name of the profile new sessions start with.
    pub default_profile: String,
    /// Quick-pick sizes in MB.
    pub sizes: Vec<f64>,
    /// Encoders the user switched off. They are never picked automatically.
    pub disabled_encoders: Vec<String>,
    pub output_mode: OutputMode,
    pub output_folder: Option<PathBuf>,
    pub history_enabled: bool,
    /// Drop history entries older than this many days. Zero keeps them.
    pub history_keep_days: u32,
    /// Videos compressed at the same time. Zero decides automatically.
    pub parallel_jobs: u32,
    /// Folder that holds `ffmpeg` and `ffprobe`, when not found automatically.
    pub ffmpeg_folder: Option<PathBuf>,
    pub watch: WatchSettings,
}

impl Default for Settings {
    fn default() -> Self {
        let base = EncodeOptions { target_mb: 10.0, ..EncodeOptions::default() };
        Settings {
            profiles: vec![
                Profile { name: "Everyday".into(), options: base.clone() },
                Profile {
                    name: "Sharper (HEVC)".into(),
                    options: EncodeOptions { codec: Codec::Hevc, effort: Effort::P5, ..base.clone() },
                },
                Profile {
                    name: "Smallest (AV1)".into(),
                    options: EncodeOptions { codec: Codec::Av1, effort: Effort::P5, container: Container::Mkv, ..base },
                },
            ],
            default_profile: "Everyday".into(),
            sizes: vec![8.0, 10.0, 25.0, 50.0, 100.0],
            disabled_encoders: vec!["libaom-av1".into()],
            output_mode: OutputMode::SameFolder,
            output_folder: None,
            history_enabled: true,
            history_keep_days: 0,
            parallel_jobs: 0,
            ffmpeg_folder: None,
            watch: WatchSettings::default(),
        }
    }
}

/// Largest size the app accepts, in MB (50 GB).
pub const MAX_TARGET_MB: f64 = 51_200.0;

fn tidy_options(options: &mut EncodeOptions) {
    if !options.target_mb.is_finite() || options.target_mb <= 0.0 {
        options.target_mb = 10.0;
    }
    options.target_mb = options.target_mb.min(MAX_TARGET_MB);
    options.audio_kbps = options.audio_kbps.clamp(16, 512);
    options.min_auto_height = options.min_auto_height.clamp(144, 2160);
    options.video_kbps = options.video_kbps.filter(|k| k.is_finite() && *k > 0.0);
    options.max_fps = options.max_fps.filter(|f| f.is_finite() && *f > 0.0);
    options.max_short_side = options.max_short_side.filter(|s| *s >= 16);
    options.max_width = options.max_width.filter(|w| *w >= 16);
    options.max_height = options.max_height.filter(|h| *h >= 16);
    if let Some(name) = &options.encoder {
        match crate::encoders::find(name) {
            Some(encoder) => options.codec = encoder.codec,
            None => options.encoder = None,
        }
    }
}

impl Settings {
    /// Repairs anything out of range, so the rest of the app can trust it.
    pub fn tidy(&mut self) {
        self.sizes.retain(|s| s.is_finite() && *s > 0.0 && *s <= MAX_TARGET_MB);
        self.sizes.sort_by(|a, b| a.total_cmp(b));
        self.sizes.dedup();
        if self.sizes.is_empty() {
            self.sizes = Settings::default().sizes;
        }

        let mut seen = std::collections::HashSet::new();
        self.profiles.retain_mut(|p| {
            p.name = p.name.trim().to_string();
            !p.name.is_empty() && seen.insert(p.name.clone())
        });
        if self.profiles.is_empty() {
            self.profiles = Settings::default().profiles;
        }
        for profile in &mut self.profiles {
            tidy_options(&mut profile.options);
        }
        if !self.profiles.iter().any(|p| p.name == self.default_profile) {
            self.default_profile = self.profiles[0].name.clone();
        }

        self.disabled_encoders.retain(|name| crate::encoders::find(name).is_some());
        self.disabled_encoders.sort();
        self.disabled_encoders.dedup();
        self.parallel_jobs = self.parallel_jobs.min(8);
        self.history_keep_days = self.history_keep_days.min(3650);
        self.watch.stable_seconds = self.watch.stable_seconds.clamp(2, 60);
        self.watch.poll_seconds = self.watch.poll_seconds.clamp(2, 300);
        if let Some(name) = &self.watch.profile {
            if !self.profiles.iter().any(|p| &p.name == name) {
                self.watch.profile = None;
            }
        }
    }

    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Options of the default profile.
    pub fn default_options(&self) -> EncodeOptions {
        self.profile(&self.default_profile)
            .or_else(|| self.profiles.first())
            .map(|p| p.options.clone())
            .unwrap_or_default()
    }
}

/// Writes `text` to `path` so that a crash never leaves half a file.
pub(crate) fn write_atomic(path: &Path, text: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ctx("could not create the settings folder")?;
    }
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, text).ctx(format!("could not write {}", temp.display()))?;
    std::fs::rename(&temp, path).ctx(format!("could not replace {}", path.display()))
}

/// Settings in memory plus their file on disk.
pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<Settings>,
}

impl SettingsStore {
    /// Loads settings, falling back to defaults when the file is missing or
    /// damaged. A damaged file is kept beside it as `settings.broken.json`.
    pub fn open(path: PathBuf) -> SettingsStore {
        let mut settings = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(settings) => settings,
                Err(e) => {
                    log::warn!("settings file is damaged ({e}); starting from defaults");
                    let _ = std::fs::rename(&path, path.with_extension("broken.json"));
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        };
        settings.tidy();
        SettingsStore { path, current: RwLock::new(settings) }
    }

    pub fn get(&self) -> Settings {
        self.current.read().expect("settings lock").clone()
    }

    /// Replaces the settings and saves them. Returns what was stored, which
    /// may differ from the input where values were out of range.
    pub fn set(&self, mut settings: Settings) -> Result<Settings> {
        settings.tidy();
        let text =
            serde_json::to_string_pretty(&settings).map_err(|e| Error::Json { what: "settings".into(), source: e })?;
        write_atomic(&self.path, &text)?;
        *self.current.write().expect("settings lock") = settings.clone();
        Ok(settings)
    }

    pub fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<Settings> {
        let mut settings = self.get();
        change(&mut settings);
        self.set(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_tidy() {
        let mut settings = Settings::default();
        let before = settings.clone();
        settings.tidy();
        assert_eq!(settings, before);
        assert_eq!(settings.default_options().target_mb, 10.0);
    }

    #[test]
    fn tidy_repairs_bad_values() {
        let mut settings = Settings {
            sizes: vec![50.0, -1.0, 8.0, 8.0, f64::NAN],
            default_profile: "gone".into(),
            disabled_encoders: vec!["nope".into(), "libx265".into()],
            parallel_jobs: 99,
            ..Settings::default()
        };
        settings.profiles[0].options.target_mb = -3.0;
        settings.profiles[0].options.encoder = Some("hevc_nvenc".into());
        settings.profiles.push(Profile { name: "Everyday".into(), options: EncodeOptions::default() });
        settings.watch.poll_seconds = 0;
        settings.tidy();
        assert_eq!(settings.sizes, vec![8.0, 50.0]);
        assert_eq!(settings.default_profile, "Everyday");
        assert_eq!(settings.disabled_encoders, vec!["libx265".to_string()]);
        assert_eq!(settings.parallel_jobs, 8);
        assert_eq!(settings.profiles.len(), 3);
        assert_eq!(settings.profiles[0].options.target_mb, 10.0);
        // A named encoder decides the codec.
        assert_eq!(settings.profiles[0].options.codec, Codec::Hevc);
        assert_eq!(settings.watch.poll_seconds, 2);
    }

    #[test]
    fn round_trips_and_survives_a_damaged_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = SettingsStore::open(path.clone());
        store.update(|s| s.sizes.push(250.0)).unwrap();
        assert!(SettingsStore::open(path.clone()).get().sizes.contains(&250.0));

        // Unknown and missing fields are tolerated.
        std::fs::write(&path, r#"{"sizes":[3],"somethingNew":true}"#).unwrap();
        let store = SettingsStore::open(path.clone());
        assert_eq!(store.get().sizes, vec![3.0]);
        assert_eq!(store.get().profiles.len(), 3);

        std::fs::write(&path, "{ not json").unwrap();
        let store = SettingsStore::open(path.clone());
        assert_eq!(store.get(), Settings::default());
        assert!(dir.path().join("settings.broken.json").exists());
    }
}
