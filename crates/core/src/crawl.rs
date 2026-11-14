//! Finding the videos in what was dropped or chosen: files stay as they are,
//! folders are searched with their subfolders.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::engine::looks_generated;
use crate::watch::is_video;

/// How many videos one drop can add. A drop of a whole drive stops here
/// instead of reading for minutes.
pub const MOST: usize = 1000;

/// What a drop or a choice turned into.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    /// The files, in the order they should appear: what was named first,
    /// and inside a folder its own videos by name, then its subfolders by name.
    pub files: Vec<PathBuf>,
    /// The search stopped at [`MOST`] videos.
    pub capped: bool,
}

/// Turns dropped or chosen paths into the files to add. A file is kept
/// whatever its type (reading it says whether it is a video). A folder adds
/// the videos in it and in its subfolders, leaving out Sub20's own results
/// (`clip-20mb.mp4`), unfinished files (`.part.`), anything hidden (a name
/// starting with a dot) and links to other folders, which could loop.
pub fn find_videos(paths: &[PathBuf]) -> Found {
    let mut found = Found::default();
    for path in paths {
        if found.capped {
            break;
        }
        if path.is_dir() {
            search(path, &mut found);
        } else {
            push(&mut found, path.clone());
        }
    }
    found
}

fn push(found: &mut Found, path: PathBuf) {
    if found.files.contains(&path) {
        return;
    }
    if found.files.len() >= MOST {
        found.capped = true;
        return;
    }
    found.files.push(path);
}

fn search(dir: &Path, found: &mut Found) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut files = Vec::new();
    let mut folders = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        // The entry's own type: a link to a folder is not followed.
        let Ok(kind) = entry.file_type() else { continue };
        let path = entry.path();
        if kind.is_dir() {
            folders.push(path);
        } else if kind.is_file() && wanted(&path) {
            files.push(path);
        }
    }
    let by_name = |a: &PathBuf, b: &PathBuf| {
        let key = |p: &PathBuf| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        key(a).cmp(&key(b))
    };
    files.sort_by(by_name);
    folders.sort_by(by_name);
    for file in files {
        push(found, file);
        if found.capped {
            return;
        }
    }
    for folder in folders {
        search(&folder, found);
        if found.capped {
            return;
        }
    }
}

/// A video someone recorded, not a result or a file still being written.
fn wanted(path: &Path) -> bool {
    if !is_video(path) {
        return false;
    }
    let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    if name.contains(".part.") {
        return false;
    }
    let stem = path.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default();
    !looks_generated(&stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    fn names(found: &Found, root: &Path) -> Vec<String> {
        found.files.iter().map(|p| p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/")).collect()
    }

    #[test]
    fn a_folder_is_searched_with_its_subfolders_in_name_order() {
        let root = tempfile::tempdir().unwrap();
        let r = root.path();
        for name in ["b.mp4", "A.mkv", "notes.txt", "Game/z.mp4", "Game/Day 2/y.mov", "game-2/x.mp4"] {
            touch(&r.join(name));
        }
        let found = find_videos(&[r.to_path_buf()]);
        assert_eq!(names(&found, r), ["A.mkv", "b.mp4", "Game/z.mp4", "Game/Day 2/y.mov", "game-2/x.mp4"]);
        assert!(!found.capped);
    }

    #[test]
    fn results_unfinished_and_hidden_files_are_left_out() {
        let root = tempfile::tempdir().unwrap();
        let r = root.path();
        for name in [
            "clip.mp4",
            "clip-20mb.mp4",
            "clip-20mb-2.mp4",
            "clip-2500k.mkv",
            ".clip.1234.part.mp4",
            ".thumbs/frame.mp4",
            ".hidden.mp4",
        ] {
            touch(&r.join(name));
        }
        assert_eq!(names(&find_videos(&[r.to_path_buf()]), r), ["clip.mp4"]);
    }

    #[test]
    fn a_named_file_is_kept_whatever_it_is_and_only_once() {
        let root = tempfile::tempdir().unwrap();
        let r = root.path();
        touch(&r.join("odd.bin"));
        touch(&r.join("v.mp4"));
        let found = find_videos(&[r.join("odd.bin"), r.to_path_buf(), r.join("v.mp4")]);
        assert_eq!(names(&found, r), ["odd.bin", "v.mp4"]);
    }

    #[test]
    fn the_search_stops_at_the_most_one_drop_can_add() {
        let root = tempfile::tempdir().unwrap();
        let r = root.path();
        for i in 0..MOST + 5 {
            touch(&r.join(format!("{i:05}.mp4")));
        }
        let found = find_videos(&[r.to_path_buf()]);
        assert_eq!(found.files.len(), MOST);
        assert!(found.capped);
    }
}
