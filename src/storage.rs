use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Exists(PathBuf),
    InvalidName(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Exists(p) => write!(f, "file exists: {}", p.display()),
            Self::InvalidName(s) => write!(f, "invalid filename: {s}"),
        }
    }
}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Hidden autosave location: ~/.config/notes-tui/autosave.txt (via ProjectDirs).
pub fn autosave_path() -> PathBuf {
    if let Some(proj) = directories::ProjectDirs::from("", "", "notes-tui") {
        proj.config_dir().join("autosave.txt")
    } else if let Some(home) = dirs_home() {
        home.join(".config").join("notes-tui").join("autosave.txt")
    } else {
        PathBuf::from("autosave.txt")
    }
}

/// Fixed export directory: ~/Downloads/notes
pub fn export_dir() -> PathBuf {
    if let Some(home) = dirs_home() {
        home.join("Downloads").join("notes")
    } else {
        PathBuf::from("notes")
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Load autosave file. Missing file -> empty buffer with one empty line.
pub fn load_autosave(path: &Path) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(s) => {
            if s.is_empty() {
                vec![String::new()]
            } else {
                s.lines().map(|l| l.to_string()).collect()
            }
        }
        Err(_) => vec![String::new()],
    }
}

/// Persist buffer to autosave path, creating parent dirs.
pub fn save_autosave(path: &Path, lines: &[String]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, lines.join("\n"))
}

/// Clean user filename: trim, reject empty / path separators, ensure .txt suffix.
pub fn sanitize_filename(input: &str) -> Result<String, String> {
    let t = input.trim();
    if t.is_empty() {
        return Err("filename is empty".to_string());
    }
    if t.contains('/') || t.contains('\\') || t.contains('\0') {
        return Err("filename must not contain / or \\".to_string());
    }
    if t == "." || t == ".." {
        return Err("invalid filename".to_string());
    }
    let mut name = t.to_string();
    if !name.to_lowercase().ends_with(".txt") {
        name.push_str(".txt");
    }
    if name.len() > 255 {
        return Err("filename too long".to_string());
    }
    Ok(name)
}

/// Write buffer to export dir. If `overwrite` is false and file exists, return SaveError::Exists.
pub fn save_export(
    dir: &Path,
    filename: &str,
    lines: &[String],
    overwrite: bool,
) -> Result<PathBuf, SaveError> {
    let clean = sanitize_filename(filename).map_err(SaveError::InvalidName)?;
    fs::create_dir_all(dir)?;
    let dest = dir.join(clean);
    if dest.exists() && !overwrite {
        return Err(SaveError::Exists(dest));
    }
    fs::write(&dest, lines.join("\n"))?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("notes-tui-test-{}-{}", std::process::id(), name));
        p
    }

    #[test]
    fn sanitize_adds_txt() {
        assert_eq!(sanitize_filename("hello").unwrap(), "hello.txt");
        assert_eq!(sanitize_filename("hello.txt").unwrap(), "hello.txt");
        assert_eq!(sanitize_filename("  notes  ").unwrap(), "notes.txt");
    }

    #[test]
    fn sanitize_rejects_bad() {
        assert!(sanitize_filename("").is_err());
        assert!(sanitize_filename("   ").is_err());
        assert!(sanitize_filename("a/b").is_err());
        assert!(sanitize_filename("..").is_err());
    }

    #[test]
    fn autosave_roundtrip() {
        let dir = tmp_path("autosave");
        let _ = fs::remove_dir_all(&dir);
        let file = dir.join("autosave.txt");
        let lines = vec!["hello".to_string(), "world 宽".to_string()];
        save_autosave(&file, &lines).unwrap();
        assert_eq!(load_autosave(&file), lines);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_returns_empty_line() {
        let p = tmp_path("missing-xyz-123.txt");
        let _ = fs::remove_file(&p);
        assert_eq!(load_autosave(&p), vec![String::new()]);
    }

    #[test]
    fn export_overwrite_guard() {
        let dir = tmp_path("export");
        let _ = fs::remove_dir_all(&dir);
        let lines = vec!["a".to_string()];
        let first = save_export(&dir, "n", &lines, false).unwrap();
        assert!(first.exists());
        assert!(matches!(
            save_export(&dir, "n", &lines, false),
            Err(SaveError::Exists(_))
        ));
        save_export(&dir, "n", &lines, true).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }
}
