//! Local workspace folder helpers — std only.
//!
//! Notes live as `.md` files under `.tinkerway-workspace/` (cwd-relative).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const WORKSPACE_DIR_NAME: &str = ".tinkerway-workspace";

/// Resolve the workspace directory under the process current working directory.
pub fn workspace_dir() -> PathBuf {
    PathBuf::from(WORKSPACE_DIR_NAME)
}

/// Create the workspace directory if it does not exist.
pub fn ensure_workspace(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)
}

/// List note filenames (`.md` only), newest first when mtime is available.
pub fn list_notes(dir: &Path) -> io::Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<(SystemTime, String)> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH);
        entries.push((modified, name));
    }

    entries.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    Ok(entries.into_iter().map(|(_, name)| name).collect())
}

/// Write `line` as a new markdown note. Returns the created filename.
pub fn write_note(dir: &Path, line: &str) -> io::Result<String> {
    ensure_workspace(dir)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "note line is empty",
        ));
    }

    let filename = unique_note_filename(dir, trimmed)?;
    let path = dir.join(&filename);
    let body = format!("{trimmed}\n");
    fs::write(&path, body)?;
    Ok(filename)
}

fn unique_note_filename(dir: &Path, line: &str) -> io::Result<String> {
    let stamp = timestamp_stamp()?;
    let slug = slugify(line);
    let base = if slug.is_empty() {
        format!("note-{stamp}")
    } else {
        format!("{stamp}-{slug}")
    };

    let mut candidate = format!("{base}.md");
    let mut n = 2u32;
    while dir.join(&candidate).exists() {
        candidate = format!("{base}-{n}.md");
        n += 1;
        if n > 10_000 {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "could not find a unique note filename",
            ));
        }
    }
    Ok(candidate)
}

fn timestamp_stamp() -> io::Result<String> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(e))?
        .as_secs();
    Ok(format!("{secs}"))
}

fn slugify(line: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in line.chars().take(40) {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            Some('-')
        } else {
            None
        };
        match mapped {
            Some('-') if prev_dash || out.is_empty() => {}
            Some('-') => {
                out.push('-');
                prev_dash = true;
            }
            Some(c) => {
                out.push(c);
                prev_dash = false;
            }
            None => {}
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_workspace() -> PathBuf {
        let mut dir = env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.push(format!("tinkerway-test-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn write_and_list_notes() {
        let dir = temp_workspace();
        let a = write_note(&dir, "hello world").unwrap();
        assert!(a.ends_with(".md"));
        assert!(dir.join(&a).is_file());

        let listed = list_notes(&dir).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], a);

        let body = fs::read_to_string(dir.join(&a)).unwrap();
        assert_eq!(body, "hello world\n");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_empty_line() {
        let dir = temp_workspace();
        let err = write_note(&dir, "   ").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_empty_missing_dir() {
        let mut dir = env::temp_dir();
        dir.push("tinkerway-missing-dir-should-not-exist");
        let _ = fs::remove_dir_all(&dir);
        assert!(list_notes(&dir).unwrap().is_empty());
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("  "), "");
    }
}
