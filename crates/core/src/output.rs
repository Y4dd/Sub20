//! Naming and placing finished files: a name the user typed, replacing the
//! original, and renaming a result afterwards.

use std::io;
use std::path::{Path, PathBuf};

use crate::error::{Error, IoContext, Result};

/// Turns what someone typed into a file name stem that works on every
/// system, or `None` when nothing usable is left. An extension matching
/// `extension` is dropped, so typing `clip.mp4` gives `clip`.
pub fn clean_name(typed: &str, extension: &str) -> Option<String> {
    let typed = typed.trim();
    let typed = typed
        .strip_suffix(&format!(".{extension}"))
        .or_else(|| typed.strip_suffix(&format!(".{}", extension.to_uppercase())))
        .unwrap_or(typed);
    let mut name: String = typed
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .collect();
    // Windows drops trailing dots and spaces, which would change the name.
    name.truncate(name.trim_end_matches(['.', ' ']).len());
    let name: String = name.trim_start().chars().take(200).collect();
    if name.is_empty() || name.chars().all(|c| c == '.') {
        return None;
    }
    // Names Windows keeps for devices, with or without an extension.
    let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && base.as_bytes()[3].is_ascii_digit());
    Some(if reserved { format!("{name}_") } else { name })
}

/// `dir/stem.ext`, or `dir/stem-2.ext` and so on when that is taken.
pub fn free_name(dir: &Path, stem: &str, extension: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{extension}"));
    if !first.exists() {
        return first;
    }
    (2..).map(|n| dir.join(format!("{stem}-{n}.{extension}"))).find(|p| !p.exists()).expect("a free name")
}

/// Where a finished file ended up.
#[derive(Debug, PartialEq)]
pub struct Placed {
    pub path: PathBuf,
    /// The original went to the Recycle Bin and the result took its place.
    pub replaced: bool,
    /// Something worth telling the user, such as an original that could not
    /// be moved to the bin.
    pub note: Option<String>,
}

/// Moves a finished `partial` file to `dir/stem.extension`.
///
/// With `original`, the result replaces it: the result is saved first under
/// a free name, then the original goes to the bin through `to_bin`, and only
/// then does the result take the wanted name. Whatever fails, nothing is
/// lost: if the bin refuses, both files stay.
pub fn place(
    partial: &Path,
    dir: &Path,
    stem: &str,
    extension: &str,
    original: Option<&Path>,
    to_bin: impl Fn(&Path) -> io::Result<()>,
) -> Result<Placed> {
    let saved = free_name(dir, stem, extension);
    std::fs::rename(partial, &saved).ctx(format!("could not save {}", saved.display()))?;
    let Some(original) = original else {
        return Ok(Placed { path: saved, replaced: false, note: None });
    };
    if let Err(error) = to_bin(original) {
        return Ok(Placed {
            path: saved,
            replaced: false,
            note: Some(format!("The original could not be moved to the Recycle Bin ({error}), so both were kept.")),
        });
    }
    let wanted = dir.join(format!("{stem}.{extension}"));
    if saved != wanted && !wanted.exists() && std::fs::rename(&saved, &wanted).is_ok() {
        return Ok(Placed { path: wanted, replaced: true, note: None });
    }
    Ok(Placed { path: saved, replaced: true, note: None })
}

/// Renames a finished file within its folder, keeping its extension.
pub fn rename(path: &Path, typed: &str) -> Result<PathBuf> {
    let extension = path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = clean_name(typed, &extension)
        .ok_or_else(|| Error::Invalid("That name has no letters a file name can use.".into()))?;
    let dir = path.parent().ok_or_else(|| Error::Invalid("That file has no folder.".into()))?;
    let target = if extension.is_empty() { dir.join(&stem) } else { dir.join(format!("{stem}.{extension}")) };
    if target == path {
        return Ok(target);
    }
    // A change of case alone is the same file on Windows and macOS.
    let same_file = target.to_string_lossy().to_lowercase() == path.to_string_lossy().to_lowercase();
    if target.exists() && !same_file {
        return Err(Error::Invalid(format!(
            "There is already a file called {} there.",
            target.file_name().unwrap_or_default().to_string_lossy()
        )));
    }
    if !path.exists() {
        return Err(Error::Invalid("That file is no longer there.".into()));
    }
    std::fs::rename(path, &target).ctx(format!("could not rename {}", path.display()))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_cleaned_for_every_system() {
        assert_eq!(clean_name("  my clip  ", "mp4").as_deref(), Some("my clip"));
        assert_eq!(clean_name("clip.mp4", "mp4").as_deref(), Some("clip"));
        assert_eq!(clean_name("clip.MP4", "mp4").as_deref(), Some("clip"));
        assert_eq!(clean_name("a/b\\c:d*e?", "mp4").as_deref(), Some("abcde"));
        assert_eq!(clean_name("trailing... ", "mp4").as_deref(), Some("trailing"));
        assert_eq!(clean_name("con", "mp4").as_deref(), Some("con_"));
        assert_eq!(clean_name("COM3", "mp4").as_deref(), Some("COM3_"));
        assert_eq!(clean_name("comet", "mp4").as_deref(), Some("comet"));
        assert_eq!(clean_name("   ", "mp4"), None);
        assert_eq!(clean_name("...", "mp4"), None);
        assert_eq!(clean_name("??", "mp4"), None);
    }

    fn bin(dir: &Path) -> impl Fn(&Path) -> io::Result<()> + '_ {
        move |file: &Path| std::fs::rename(file, dir.join(file.file_name().unwrap()))
    }

    #[test]
    fn replacing_takes_the_originals_name_and_bins_it() {
        let dir = tempfile::tempdir().unwrap();
        let trash = tempfile::tempdir().unwrap();
        let original = dir.path().join("clip.mp4");
        let partial = dir.path().join(".clip.abc.part.mp4");
        std::fs::write(&original, b"big original").unwrap();
        std::fs::write(&partial, b"small").unwrap();

        let placed = place(&partial, dir.path(), "clip", "mp4", Some(&original), bin(trash.path())).unwrap();
        assert_eq!(placed, Placed { path: original.clone(), replaced: true, note: None });
        assert_eq!(std::fs::read(&original).unwrap(), b"small");
        assert_eq!(std::fs::read(trash.path().join("clip.mp4")).unwrap(), b"big original");
        assert!(!partial.exists());
    }

    #[test]
    fn replacing_with_a_new_name_or_extension() {
        let dir = tempfile::tempdir().unwrap();
        let trash = tempfile::tempdir().unwrap();
        let original = dir.path().join("clip.mov");
        let partial = dir.path().join(".clip.abc.part.mp4");
        std::fs::write(&original, b"original").unwrap();
        std::fs::write(&partial, b"small").unwrap();

        let placed = place(&partial, dir.path(), "for discord", "mp4", Some(&original), bin(trash.path())).unwrap();
        assert_eq!(placed.path, dir.path().join("for discord.mp4"));
        assert!(placed.replaced);
        assert!(!original.exists());
    }

    #[test]
    fn a_refusing_bin_keeps_both() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("clip.mp4");
        let partial = dir.path().join(".clip.abc.part.mp4");
        std::fs::write(&original, b"original").unwrap();
        std::fs::write(&partial, b"small").unwrap();

        let refuse = |_: &Path| Err(io::Error::other("no bin here"));
        let placed = place(&partial, dir.path(), "clip", "mp4", Some(&original), refuse).unwrap();
        assert_eq!(placed.path, dir.path().join("clip-2.mp4"));
        assert!(!placed.replaced);
        assert!(placed.note.unwrap().contains("no bin here"));
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
    }

    #[test]
    fn renaming_keeps_the_extension_and_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("clip-20mb.mp4");
        std::fs::write(&file, b"x").unwrap();
        std::fs::write(dir.path().join("taken.mp4"), b"y").unwrap();

        let renamed = rename(&file, "for discord.mp4").unwrap();
        assert_eq!(renamed, dir.path().join("for discord.mp4"));
        assert!(renamed.exists() && !file.exists());
        assert!(rename(&renamed, "taken").unwrap_err().to_string().contains("already a file"));
        assert!(rename(&renamed, "  ").is_err());
        assert_eq!(rename(&renamed, "For Discord").unwrap(), dir.path().join("For Discord.mp4"));
    }
}
