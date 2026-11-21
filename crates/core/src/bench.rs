//! The bench as it was left (plan 7's 7.13): the clips not yet compressed,
//! with their trim, typed name and "leave out", kept in `bench.json` so they
//! are there at the next launch or when the window is built again. The
//! window saves it a moment after it changes; the shell adds to it the clips
//! the folder watch puts on the bench while there is no window.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::job::Trim;
use crate::settings::{read_json, write_atomic};

/// One clip waiting on the bench.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchClip {
    pub path: PathBuf,
    /// The kept part; `None` keeps all of it.
    #[serde(default)]
    pub trim: Option<Trim>,
    /// What to call the result, as typed.
    #[serde(default)]
    pub output_name: Option<String>,
    #[serde(default)]
    pub left_out: bool,
}

impl BenchClip {
    pub fn new(path: PathBuf) -> BenchClip {
        BenchClip { path, trim: None, output_name: None, left_out: false }
    }
}

/// The saved bench, in order, each file once, leaving out files that are no
/// longer there. A missing or damaged file is an empty bench.
pub fn load(file: &Path) -> Vec<BenchClip> {
    let Ok(text) = read_json(file) else { return Vec::new() };
    let clips: Vec<BenchClip> = serde_json::from_str(&text).unwrap_or_else(|e| {
        log::warn!("the saved bench is damaged ({e}); starting empty");
        Vec::new()
    });
    let mut seen = HashSet::new();
    clips.into_iter().filter(|clip| clip.path.is_file() && seen.insert(clip.path.clone())).collect()
}

/// Replaces the saved bench.
pub fn save(file: &Path, clips: &[BenchClip]) -> Result<()> {
    let text = serde_json::to_string_pretty(clips).map_err(|e| Error::Json { what: "the bench".into(), source: e })?;
    write_atomic(file, &text)
}

/// Adds clips to the end of the saved bench, whole and untrimmed, leaving
/// out any already on it.
pub fn add(file: &Path, paths: &[PathBuf]) -> Result<()> {
    let mut clips = load(file);
    for path in paths {
        if !clips.iter().any(|clip| &clip.path == path) {
            clips.push(BenchClip::new(path.clone()));
        }
    }
    save(file, &clips)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_drops_files_that_are_gone() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bench.json");
        assert!(load(&file).is_empty(), "no file is an empty bench");

        let (a, b, gone) = (dir.path().join("a.mp4"), dir.path().join("b.mp4"), dir.path().join("gone.mp4"));
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let clips = vec![
            BenchClip {
                path: b.clone(),
                trim: Some(Trim { start: 1.5, end: 9.0 }),
                output_name: Some("the play".into()),
                left_out: true,
            },
            BenchClip::new(gone),
            BenchClip::new(a.clone()),
        ];
        save(&file, &clips).unwrap();
        let back = load(&file);
        assert_eq!(back, vec![clips[0].clone(), clips[2].clone()], "in order, without the missing file");

        std::fs::write(&file, "{ not json").unwrap();
        assert!(load(&file).is_empty());
    }

    #[test]
    fn adding_keeps_what_is_there_and_skips_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bench.json");
        let (a, b) = (dir.path().join("a.mp4"), dir.path().join("b.mp4"));
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let trimmed = BenchClip { trim: Some(Trim { start: 2.0, end: 4.0 }), ..BenchClip::new(a.clone()) };
        save(&file, std::slice::from_ref(&trimmed)).unwrap();

        add(&file, &[a.clone(), b.clone(), b.clone()]).unwrap();
        assert_eq!(load(&file), vec![trimmed, BenchClip::new(b)]);
    }

    #[test]
    fn reads_a_file_written_by_the_window() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bench.json");
        let clip = dir.path().join("clip.mp4");
        std::fs::write(&clip, b"x").unwrap();
        let path = serde_json::to_string(&clip).unwrap();
        let text = format!(
            r#"[{{"path":{path},"trim":{{"start":1,"end":2}},"outputName":null,"leftOut":false}},{{"path":{path}}}]"#
        );
        std::fs::write(&file, text).unwrap();
        assert_eq!(load(&file), vec![BenchClip { trim: Some(Trim { start: 1.0, end: 2.0 }), ..BenchClip::new(clip) }]);
    }
}
