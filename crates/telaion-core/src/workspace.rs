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

    for entry in walker.flatten() {
        if entry.path() != dir {
            return true;
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
            let children = scan_workspace_recursive(vault_root, &entry.path, depth + 1, max_depth)?;
            entry.children = Some(children);
        }
    }

    Ok(entries)
}

/// Flattened list of all non-ignored files in the workspace for fast fuzzy searching / quick open
pub fn list_all_files(vault_root: &Path) -> std::io::Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    let walker = WalkBuilder::new(vault_root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path == vault_root {
            continue;
        }

        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        if is_dir {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let relative_path = path
            .strip_prefix(vault_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let extension = path.extension().map(|e| e.to_string_lossy().to_string());

        files.push(FileEntry {
            name,
            path: path.to_path_buf(),
            relative_path,
            is_dir: false,
            extension,
            has_children: false,
            children: None,
        });
    }

    files.sort_by(|a, b| a.relative_path.to_lowercase().cmp(&b.relative_path.to_lowercase()));
    Ok(files)
}

/// Safely resolve a path within `vault_root`.
/// Returns an error if the path attempts to escape `vault_root`.
pub fn resolve_in_vault(vault_root: &Path, target: &Path) -> std::io::Result<PathBuf> {
    let canonical_root = vault_root
        .canonicalize()
        .unwrap_or_else(|_| vault_root.to_path_buf());

    let candidate = if target.is_absolute() {
        target.to_path_buf()
    } else {
        canonical_root.join(target)
    };

    // Lexically normalize to eliminate `.` and `..` without requiring the target file to exist yet
    let mut normalized = PathBuf::new();
    for comp in candidate.components() {
        match comp {
            std::path::Component::Prefix(p) => normalized.push(p.as_os_str()),
            std::path::Component::RootDir => normalized.push(comp.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Vault外のパスへのアクセスは許可されていません",
                    ));
                }
            }
            std::path::Component::Normal(c) => normalized.push(c),
        }
    }

    // Check boundary
    if !normalized.starts_with(&canonical_root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Vault外のパスへのアクセスは許可されていません",
        ));
    }

    // If the path actually exists, also verify with canonicalize() to protect against symlink traversal
    if normalized.exists() {
        let canon = normalized.canonicalize()?;
        if !canon.starts_with(&canonical_root) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Vault外のパスへのアクセスは許可されていません",
            ));
        }
        Ok(canon)
    } else {
        // If it doesn't exist yet, verify parent doesn't escape via symlink if parent exists
        if let Some(parent) = normalized.parent() {
            if parent.exists() {
                let canon_parent = parent.canonicalize()?;
                if !canon_parent.starts_with(&canonical_root) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Vault外のパスへのアクセスは許可されていません",
                    ));
                }
            }
        }
        Ok(normalized)
    }
}

/// Construct a `FileEntry` from a path within `vault_root`.
pub fn entry_from_path(vault_root: &Path, path: &Path) -> std::io::Result<FileEntry> {
    let canonical_root = vault_root
        .canonicalize()
        .unwrap_or_else(|_| vault_root.to_path_buf());
    let is_dir = path.is_dir();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let relative_path = path
        .strip_prefix(&canonical_root)
        .or_else(|_| path.strip_prefix(vault_root))
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

    Ok(FileEntry {
        name,
        path: path.to_path_buf(),
        relative_path,
        is_dir,
        extension,
        has_children,
        children: None,
    })
}

/// Create a new file within `vault_root`.
pub fn create_file(
    vault_root: &Path,
    relative_path: &Path,
    initial_content: Option<&str>,
) -> std::io::Result<FileEntry> {
    if relative_path.as_os_str().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "ファイル名が指定されていません",
        ));
    }
    let full_path = resolve_in_vault(vault_root, relative_path)?;
    if full_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("ファイルが既に存在します: {}", full_path.display()),
        ));
    }
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, initial_content.unwrap_or(""))?;
    entry_from_path(vault_root, &full_path)
}

/// Create a new directory within `vault_root`.
pub fn create_directory(vault_root: &Path, relative_path: &Path) -> std::io::Result<FileEntry> {
    if relative_path.as_os_str().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "ディレクトリ名が指定されていません",
        ));
    }
    let full_path = resolve_in_vault(vault_root, relative_path)?;
    if full_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("ディレクトリが既に存在します: {}", full_path.display()),
        ));
    }
    std::fs::create_dir_all(&full_path)?;
    entry_from_path(vault_root, &full_path)
}

/// Rename an existing file or directory within `vault_root`.
pub fn rename_entry(
    vault_root: &Path,
    old_relative: &Path,
    new_name_or_path: &str,
) -> std::io::Result<FileEntry> {
    let trimmed = new_name_or_path.trim();
    if trimmed.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "新しい名前が指定されていません",
        ));
    }

    let old_full = resolve_in_vault(vault_root, old_relative)?;
    if !old_full.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("対象が見つかりません: {}", old_full.display()),
        ));
    }

    let canonical_root = vault_root
        .canonicalize()
        .unwrap_or_else(|_| vault_root.to_path_buf());
    if old_full == canonical_root {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Vaultルート自体をリネームすることはできません",
        ));
    }

    let new_path_candidate = Path::new(trimmed);
    let new_full = if new_path_candidate.components().count() == 1 {
        // Just a filename, place in the same parent directory
        old_full.parent().unwrap_or(&canonical_root).join(trimmed)
    } else {
        resolve_in_vault(vault_root, new_path_candidate)?
    };

    if new_full.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("変更先の名前が既に存在します: {}", new_full.display()),
        ));
    }

    if let Some(parent) = new_full.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::rename(&old_full, &new_full)?;
    entry_from_path(vault_root, &new_full)
}

/// Move a file or directory `source_relative` into directory `dest_dir_relative` within `vault_root`.
pub fn move_entry(
    vault_root: &Path,
    source_relative: &Path,
    dest_dir_relative: &Path,
) -> std::io::Result<FileEntry> {
    let source_full = resolve_in_vault(vault_root, source_relative)?;
    if !source_full.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("移動元が見つかりません: {}", source_full.display()),
        ));
    }

    let canonical_root = vault_root
        .canonicalize()
        .unwrap_or_else(|_| vault_root.to_path_buf());

    if source_full == canonical_root {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Vaultルート自体を移動することはできません",
        ));
    }

    let dest_dir_full = resolve_in_vault(vault_root, dest_dir_relative)?;
    if !dest_dir_full.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            format!("移動先がディレクトリではありません: {}", dest_dir_full.display()),
        ));
    }

    // Security & sanity check: Cannot move directory into itself or into its own descendant
    if source_full.is_dir() && (dest_dir_full == source_full || dest_dir_full.starts_with(&source_full)) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "ディレクトリを自身またはその配下のディレクトリに移動することはできません",
        ));
    }

    let file_name = match source_full.file_name() {
        Some(name) => name,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "無効な移動元ファイル名です",
            ));
        }
    };

    let target_destination = dest_dir_full.join(file_name);

    if target_destination == source_full {
        return entry_from_path(vault_root, &source_full);
    }

    if target_destination.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "移動先に同名のファイルまたはフォルダが既に存在します: {}",
                target_destination.display()
            ),
        ));
    }

    std::fs::rename(&source_full, &target_destination)?;
    entry_from_path(vault_root, &target_destination)
}

/// Delete a file or directory within `vault_root`.
pub fn delete_entry(vault_root: &Path, target_relative: &Path) -> std::io::Result<()> {
    let target_full = resolve_in_vault(vault_root, target_relative)?;
    let canonical_root = vault_root
        .canonicalize()
        .unwrap_or_else(|_| vault_root.to_path_buf());
    if target_full == canonical_root {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Vaultルート自体を削除することはできません",
        ));
    }
    if !target_full.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("削除対象が見つかりません: {}", target_full.display()),
        ));
    }

    if target_full.is_dir() {
        std::fs::remove_dir_all(&target_full)?;
    } else {
        std::fs::remove_file(&target_full)?;
    }
    Ok(())
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
        fs::write(
            temp_dir.join("visible_dir").join("readme.tmt"),
            "=[ hello ]",
        )
        .unwrap();
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

    #[test]
    fn test_file_and_directory_operations() {
        let temp_dir = std::env::temp_dir().join("telaion_test_ops");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // 1. Create file
        let file_entry = create_file(&temp_dir, Path::new("test.tmt"), Some("=[ Test ]=")).unwrap();
        assert_eq!(file_entry.name, "test.tmt");
        assert_eq!(file_entry.relative_path, "test.tmt");
        assert!(!file_entry.is_dir);
        assert_eq!(fs::read_to_string(&file_entry.path).unwrap(), "=[ Test ]=");

        // Cannot create same file again
        assert!(create_file(&temp_dir, Path::new("test.tmt"), None).is_err());

        // 2. Create directory
        let dir_entry = create_directory(&temp_dir, Path::new("docs")).unwrap();
        assert_eq!(dir_entry.name, "docs");
        assert!(dir_entry.is_dir);

        // 3. Create nested file
        let nested = create_file(&temp_dir, Path::new("docs/intro.tmt"), Some("intro")).unwrap();
        assert_eq!(nested.name, "intro.tmt");
        assert_eq!(nested.relative_path, "docs/intro.tmt");

        // 4. Rename file
        let renamed = rename_entry(&temp_dir, Path::new("test.tmt"), "renamed.tmt").unwrap();
        assert_eq!(renamed.name, "renamed.tmt");
        assert_eq!(renamed.relative_path, "renamed.tmt");
        assert!(!temp_dir.join("test.tmt").exists());
        assert!(temp_dir.join("renamed.tmt").exists());

        // 5. Delete file
        delete_entry(&temp_dir, Path::new("renamed.tmt")).unwrap();
        assert!(!temp_dir.join("renamed.tmt").exists());

        // 6. Delete directory
        delete_entry(&temp_dir, Path::new("docs")).unwrap();
        assert!(!temp_dir.join("docs").exists());

        // 7. Security: Path traversal prevention
        assert!(resolve_in_vault(&temp_dir, Path::new("../outside.txt")).is_err());
        assert!(create_file(&temp_dir, Path::new("../escaped.txt"), None).is_err());
        assert!(delete_entry(&temp_dir, Path::new("")).is_err()); // Cannot delete vault root

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_list_all_files() {
        let temp_dir = std::env::temp_dir().join("telaion_test_quick_open");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        fs::write(temp_dir.join(".gitignore"), "*.log\nignored/\n").unwrap();
        fs::write(temp_dir.join("root.tmt"), "root").unwrap();
        fs::create_dir_all(temp_dir.join("docs")).unwrap();
        fs::write(temp_dir.join("docs").join("guide.tmt"), "guide").unwrap();
        fs::create_dir_all(temp_dir.join("ignored")).unwrap();
        fs::write(temp_dir.join("ignored").join("skip.tmt"), "skip").unwrap();
        fs::write(temp_dir.join("test.log"), "log").unwrap();

        let files = list_all_files(&temp_dir).unwrap();
        let rel_paths: Vec<String> = files.into_iter().map(|f| f.relative_path).collect();

        assert!(rel_paths.contains(&"root.tmt".to_string()));
        assert!(rel_paths.contains(&"docs/guide.tmt".to_string()));
        assert!(!rel_paths.contains(&"ignored/skip.tmt".to_string()));
        assert!(!rel_paths.contains(&"test.log".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_move_entry() {
        let temp_dir = std::env::temp_dir().join("telaion_test_move");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Setup folders and files
        let _ = create_file(&temp_dir, Path::new("file.tmt"), Some("hello")).unwrap();
        let _ = create_directory(&temp_dir, Path::new("sub1")).unwrap();
        let _ = create_directory(&temp_dir, Path::new("sub2")).unwrap();
        let _ = create_directory(&temp_dir, Path::new("sub1/nested")).unwrap();

        // 1. Move file into sub1
        let moved = move_entry(&temp_dir, Path::new("file.tmt"), Path::new("sub1")).unwrap();
        assert_eq!(moved.relative_path, "sub1/file.tmt");
        assert!(!temp_dir.join("file.tmt").exists());
        assert!(temp_dir.join("sub1/file.tmt").exists());

        // 2. Move file from sub1 back to root (".")
        let moved_root = move_entry(&temp_dir, Path::new("sub1/file.tmt"), Path::new(".")).unwrap();
        assert_eq!(moved_root.relative_path, "file.tmt");
        assert!(temp_dir.join("file.tmt").exists());

        // 3. Move directory sub2 into sub1
        let moved_dir = move_entry(&temp_dir, Path::new("sub2"), Path::new("sub1")).unwrap();
        assert_eq!(moved_dir.relative_path, "sub1/sub2");
        assert!(temp_dir.join("sub1/sub2").is_dir());

        // 4. Boundary and sanity checks
        // Cannot move directory into its own descendant
        assert!(move_entry(&temp_dir, Path::new("sub1"), Path::new("sub1/nested")).is_err());
        assert!(move_entry(&temp_dir, Path::new("sub1"), Path::new("sub1")).is_err());

        // Cannot move vault root
        assert!(move_entry(&temp_dir, Path::new("."), Path::new("sub1")).is_err());

        // Traversal rejection
        assert!(move_entry(&temp_dir, Path::new("../outside"), Path::new("sub1")).is_err());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
