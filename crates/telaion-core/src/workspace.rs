use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub relative_path: String,
    pub is_dir: bool,
    pub extension: Option<String>,
    pub has_children: bool,
    pub children: Option<Vec<FileEntry>>,
}

/// List immediate children of `target_dir` within `vault_root`.
/// Respects `.gitignore`, `.ignore`, and hidden files.
pub fn list_directory(vault_root: &Path, target_dir: &Path) -> std::io::Result<Vec<FileEntry>> {
    if !target_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let walker = WalkBuilder::new(target_dir)
        .max_depth(Some(1))
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Walk error: {err}");
                continue;
            }
        };

        let path = entry.path();
        // Skip target_dir itself (depth 0)
        if path == target_dir {
            continue;
        }

        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let name = entry.file_name().to_string_lossy().to_string();

        let relative_path = path
            .strip_prefix(vault_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let extension = if is_dir {
            None
        } else {
            path.extension().map(|e| e.to_string_lossy().to_string())
        };

        let has_children = if is_dir {
            check_has_children(path)
        } else {
            false
        };

        entries.push(FileEntry {
            name,
            path: path.to_path_buf(),
            relative_path,
            is_dir,
            extension,
            has_children,
            children: None,
        });
    }

    sort_entries(&mut entries);
    Ok(entries)
}

/// Check if a directory has any non-ignored children
fn check_has_children(dir: &Path) -> bool {
    let walker = WalkBuilder::new(dir)
        .max_depth(Some(1))
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .build();

    for result in walker {
        if let Ok(entry) = result {
            if entry.path() != dir {
                return true;
            }
        }
    }
    false
}

/// Recursively scan an entire workspace (up to max depth 10) respecting gitignore
pub fn scan_workspace(root: &Path) -> std::io::Result<Vec<FileEntry>> {
    scan_workspace_recursive(root, root, 0, 10)
}

fn scan_workspace_recursive(
    vault_root: &Path,
    current_dir: &Path,
    depth: usize,
    max_depth: usize,
) -> std::io::Result<Vec<FileEntry>> {
    if depth > max_depth || !current_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = list_directory(vault_root, current_dir)?;
    for entry in &mut entries {
        if entry.is_dir && entry.has_children {
            let children =
                scan_workspace_recursive(vault_root, &entry.path, depth + 1, max_depth)?;
            entry.children = Some(children);
        }
    }

    Ok(entries)
}

fn sort_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_list_directory_respects_gitignore() {
        let temp_dir = std::env::temp_dir().join("telaion_test_workspace");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create gitignore
        fs::write(temp_dir.join(".gitignore"), "ignored_dir/\n*.tmp\n").unwrap();

        // Create files and dirs
        fs::create_dir_all(temp_dir.join("visible_dir")).unwrap();
        fs::write(temp_dir.join("visible_dir").join("readme.tmt"), "=[ hello ]").unwrap();
        fs::create_dir_all(temp_dir.join("ignored_dir")).unwrap();
        fs::write(temp_dir.join("ignored_dir").join("secret.tmt"), "secret").unwrap();
        fs::write(temp_dir.join("temp.tmp"), "temp").unwrap();
        fs::write(temp_dir.join("root.tmt"), "=[ root ]").unwrap();

        let entries = list_directory(&temp_dir, &temp_dir).unwrap();
        let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();

        assert!(names.contains(&"visible_dir".to_string()));
        assert!(names.contains(&"root.tmt".to_string()));
        assert!(!names.contains(&"ignored_dir".to_string()));
        assert!(!names.contains(&"temp.tmp".to_string()));

        let visible_dir = entries.iter().find(|e| e.name == "visible_dir").unwrap();
        assert!(visible_dir.is_dir);
        assert!(visible_dir.has_children);

        let sub_entries = list_directory(&temp_dir, &visible_dir.path).unwrap();
        assert_eq!(sub_entries.len(), 1);
        assert_eq!(sub_entries[0].name, "readme.tmt");
        assert_eq!(sub_entries[0].relative_path, "visible_dir/readme.tmt");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
