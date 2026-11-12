//! A record of finished compressions.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::job::{JobResult, Origin};
use crate::settings::write_atomic;

/// Entries beyond this are dropped, oldest first.
const LIMIT: usize = 1000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// Unix time the job finished.
    pub finished_at: u64,
    pub input: PathBuf,
    pub trimmed: bool,
    #[serde(default)]
    pub origin: Origin,
    pub result: JobResult,
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub struct History {
    path: PathBuf,
    entries: Mutex<Vec<Entry>>,
}

impl History {
    pub fn open(path: PathBuf) -> History {
        let entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<Vec<Entry>>(&text).ok())
            .unwrap_or_default();
        History { path, entries: Mutex::new(entries) }
    }

    fn save(&self, entries: &[Entry]) -> Result<()> {
        let text = serde_json::to_string(entries).map_err(|e| Error::Json { what: "history".into(), source: e })?;
        write_atomic(&self.path, &text)
    }

    /// Newest first.
    pub fn list(&self) -> Vec<Entry> {
        let mut entries = self.entries.lock().expect("history lock").clone();
        entries.reverse();
        entries
    }

    pub fn add(&self, entry: Entry) -> Result<()> {
        let mut entries = self.entries.lock().expect("history lock");
        entries.push(entry);
        let extra = entries.len().saturating_sub(LIMIT);
        entries.drain(..extra);
        self.save(&entries)
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        let mut entries = self.entries.lock().expect("history lock");
        entries.retain(|e| e.id != id);
        self.save(&entries)
    }

    pub fn clear(&self) -> Result<()> {
        let mut entries = self.entries.lock().expect("history lock");
        entries.clear();
        self.save(&entries)
    }

    /// Drops entries older than `days`. Zero keeps everything.
    pub fn prune(&self, days: u32) -> Result<usize> {
        if days == 0 {
            return Ok(0);
        }
        let cutoff = now().saturating_sub(u64::from(days) * 86_400);
        let mut entries = self.entries.lock().expect("history lock");
        let before = entries.len();
        entries.retain(|e| e.finished_at >= cutoff);
        let dropped = before - entries.len();
        if dropped > 0 {
            self.save(&entries)?;
        }
        Ok(dropped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, finished_at: u64) -> Entry {
        Entry {
            id: id.into(),
            finished_at,
            input: PathBuf::from("/videos/clip.mov"),
            trimmed: false,
            origin: Origin::Manual,
            result: JobResult {
                output: PathBuf::from("/videos/clip-10mb.mp4"),
                size_bytes: 10_000_000,
                original_bytes: 80_000_000,
                duration: 30.0,
                width: 1280,
                height: 720,
                video_kbps: 2500,
                audio_kbps: 128,
                encoder: "libx264".into(),
                hardware: false,
                fell_back_from: None,
                attempts: 1,
                elapsed_seconds: 12.0,
                target_mb: Some(10.0),
            },
        }
    }

    #[test]
    fn add_list_remove_prune_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let history = History::open(path.clone());
        history.add(entry("old", now() - 10 * 86_400)).unwrap();
        history.add(entry("new", now())).unwrap();
        assert_eq!(history.list().iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["new", "old"]);

        let reopened = History::open(path);
        assert_eq!(reopened.list().len(), 2);
        assert_eq!(reopened.prune(0).unwrap(), 0);
        assert_eq!(reopened.prune(7).unwrap(), 1);
        assert_eq!(reopened.list()[0].id, "new");
        reopened.remove("new").unwrap();
        assert!(reopened.list().is_empty());
    }
}
