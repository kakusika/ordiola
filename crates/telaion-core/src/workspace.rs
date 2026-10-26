use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Option<Vec<FileEntry>>,
}

/// Recursively scan a workspace directory for files and folders
pub fn scan_workspace(root: &Path) -> std::io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    if !root.is_dir() {
        return Ok(entries);
    }

    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files/directories like .git
        if name.starts_with('.') {
            continue;
        }

        let is_dir = path.is_dir();
        let children = if is_dir {
            Some(scan_workspace(&path)?)
        } else {
            None
        };

        entries.push(FileEntry {
            name,
            path,
            is_dir,
            children,
        });
    }

    entries.sort_by(|a, b| {
        // Directories first, then alphabetical
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}
