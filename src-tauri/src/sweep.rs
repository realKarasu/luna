//! Cleaning, the part every system shares: what the Clean page lists, and measuring or emptying
//! a folder without ever stepping outside it.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// One line of the Clean page: a kind of leftover and how much of it there is.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanItem {
    pub id: String,
    /// "system", "browsers", "apps" or "dev".
    pub group: String,
    pub title: String,
    pub desc: String,
    pub bytes: u64,
    pub files: u64,
    /// Ticked when the page opens.
    pub suggested: bool,
    /// Only Luna running as administrator can clean it, and it does not right now.
    pub needs_admin: bool,
    /// Name of the program that is open and keeps some of these files in use.
    pub open: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub items: Vec<CleanItem>,
    /// Luna runs as administrator.
    pub admin: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub freed_bytes: u64,
    pub files: u64,
    pub skipped: u64,
}

/// A folder whose content can go.
pub struct Spot {
    pub dir: PathBuf,
    /// Files changed more recently than this stay.
    pub min_age: Duration,
    /// Look inside sub-folders too.
    pub deep: bool,
    /// Which file names count. Everything by default.
    pub only: fn(&str) -> bool,
}

fn any(_: &str) -> bool {
    true
}

pub const DAY: Duration = Duration::from_secs(24 * 60 * 60);

impl Spot {
    /// Everything inside `dir`, at any depth.
    pub fn all(dir: PathBuf) -> Spot {
        Spot { dir, min_age: Duration::ZERO, deep: true, only: any }
    }

    /// Everything inside `dir` that is older than `min_age`.
    pub fn older(dir: PathBuf, min_age: Duration) -> Spot {
        Spot { dir, min_age, deep: true, only: any }
    }

    /// Only the files right in `dir` whose name passes `only`.
    pub fn files(dir: PathBuf, only: fn(&str) -> bool) -> Spot {
        Spot { dir, min_age: Duration::ZERO, deep: false, only }
    }

    /// A drive root or a missing variable would give a short path: never clean those.
    fn sane(&self) -> bool {
        self.dir.is_absolute() && self.dir.components().count() >= 3
    }

    pub fn exists(&self) -> bool {
        self.sane() && self.dir.is_dir()
    }
}

/// Walks a spot. Links and junctions are never followed, so nothing outside the folder is seen.
/// `file` gets each matching file with its size; folders come back deepest first.
fn walk(spot: &Spot, mut file: impl FnMut(&Path, u64)) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if !spot.exists() {
        return dirs;
    }
    let cutoff = SystemTime::now().checked_sub(spot.min_age).unwrap_or(SystemTime::UNIX_EPOCH);
    let mut stack = vec![spot.dir.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
            let ft = meta.file_type();
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                if spot.deep {
                    dirs.push(path.clone());
                    stack.push(path);
                }
                continue;
            }
            let name = entry.file_name();
            if !(spot.only)(&name.to_string_lossy()) {
                continue;
            }
            let old = spot.min_age.is_zero() || meta.modified().map(|m| m < cutoff).unwrap_or(false);
            if old {
                file(&path, meta.len());
            }
        }
    }
    dirs.reverse();
    dirs
}

/// Size and number of the files that would go.
pub fn measure(spots: &[Spot]) -> (u64, u64) {
    let (mut bytes, mut files) = (0, 0);
    for spot in spots {
        walk(spot, |_, len| {
            bytes += len;
            files += 1;
        });
    }
    (bytes, files)
}

/// Deletes the files, then the folders left empty. The spot folders themselves stay.
/// Files in use are skipped and counted.
pub fn clear(spots: &[Spot], res: &mut CleanResult) {
    for spot in spots {
        let mut doomed = Vec::new();
        let dirs = walk(spot, |path, len| doomed.push((path.to_path_buf(), len)));
        for (path, len) in doomed {
            if std::fs::remove_file(&path).is_ok() {
                res.freed_bytes += len;
                res.files += 1;
            } else {
                res.skipped += 1;
            }
        }
        for dir in dirs {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("luna-sweep-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("one.tmp"), [0u8; 10]).unwrap();
        fs::write(dir.join("a/two.log"), [0u8; 20]).unwrap();
        fs::write(dir.join("a/b/three.tmp"), [0u8; 30]).unwrap();
        dir
    }

    #[test]
    fn measures_and_clears_everything_but_the_folder() {
        let dir = scratch("all");
        assert_eq!(measure(&[Spot::all(dir.clone())]), (60, 3));
        let mut res = CleanResult::default();
        clear(&[Spot::all(dir.clone())], &mut res);
        assert_eq!((res.freed_bytes, res.files, res.skipped), (60, 3, 0));
        assert!(dir.is_dir());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn shallow_spots_filter_names_and_stay_on_top() {
        let dir = scratch("files");
        let spot = Spot::files(dir.clone(), |n| n.ends_with(".tmp"));
        assert_eq!(measure(&[spot]), (10, 1));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn new_files_stay() {
        let dir = scratch("age");
        assert_eq!(measure(&[Spot::older(dir.clone(), DAY)]), (0, 0));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn short_or_relative_paths_are_ignored() {
        assert!(!Spot::all(PathBuf::from("C:\\")).exists());
        assert!(!Spot::all(PathBuf::from("/")).exists());
        assert!(!Spot::all(PathBuf::from("Temp")).exists());
        assert_eq!(measure(&[Spot::all(PathBuf::from("/"))]), (0, 0));
    }
}
