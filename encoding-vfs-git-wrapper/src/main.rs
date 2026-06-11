//! Transparent git wrapper for encoding-vfs.
//!
//! This wrapper automatically detects if the current directory is within a VFS mount
//! and redirects git commands to the source directory.
//!
//! It reads mount information from `~/.encoding-vfs/mounts.json` which is maintained
//! by the encoding-vfs process.

use encoding_vfs_core::MountsRegistry;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

fn main() {
    // Get current directory
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("git-wrapper: Failed to get current directory: {}", e);
            process::exit(1);
        }
    };

    // Load mounts registry
    let registry = match MountsRegistry::load() {
        Ok(r) => r,
        Err(_e) => {
            // No mounts file or error reading it - just run git normally
            return exec_git_in_dir(&current_dir, &collect_git_args());
        }
    };

    // Find matching mount for current directory
    let mount_info = match registry.find_by_path(&current_dir) {
        Some(info) => info.clone(),
        None => {
            // Not in a VFS mount - run git normally
            return exec_git_in_dir(&current_dir, &collect_git_args());
        }
    };

    // Calculate relative path from mount point
    let mount_point = PathBuf::from(&mount_info.mount_point);

    // Normalize paths for comparison (especially on Windows where "Y:" vs "Y:\" matters)
    let rel_path = normalize_strip_prefix(&current_dir, &mount_point);
    let rel_path = match rel_path {
        Some(p) => p,
        None => {
            // Path doesn't start with mount point - run git normally
            return exec_git_in_dir(&current_dir, &collect_git_args());
        }
    };

    // Build source path
    // Handle the case where rel_path is the root directory (empty or just separators)
    let source_path = if rel_path.as_os_str().is_empty()
        || rel_path.as_os_str() == "\\"
        || rel_path.as_os_str() == "/"
    {
        mount_info.source.clone()
    } else {
        mount_info.source.join(&rel_path)
    };

    // Check if source path exists
    if !source_path.exists() {
        eprintln!(
            "git-wrapper: Source path does not exist: {}",
            source_path.display()
        );
        process::exit(1);
    }

    // Execute git in source directory
    exec_git_in_dir(&source_path, &collect_git_args());
}

/// Normalize paths and strip the prefix, handling Windows drive letter variations.
///
/// On Windows, "Y:" and "Y:\" are equivalent but PathBuf::strip_prefix doesn't
/// recognize them as the same. This function handles that case.
fn normalize_strip_prefix(path: &Path, prefix: &Path) -> Option<PathBuf> {
    // First try direct strip_prefix
    if let Ok(rel) = path.strip_prefix(prefix) {
        return Some(rel.to_path_buf());
    }

    // On Windows, try adding/removing trailing separator
    #[cfg(target_os = "windows")]
    {
        let prefix_str = prefix.to_string_lossy();

        // If prefix is "Y:" and path is "Y:\something"
        if !prefix_str.ends_with('\\') && !prefix_str.ends_with('/') {
            let prefix_with_sep = format!("{}\\", prefix_str);
            let prefix_path = PathBuf::from(&prefix_with_sep);
            if let Ok(rel) = path.strip_prefix(&prefix_path) {
                return Some(rel.to_path_buf());
            }
        }

        // If prefix is "Y:\" and path is "Y:something" (unlikely but handle it)
        if prefix_str.ends_with('\\') || prefix_str.ends_with('/') {
            let prefix_without_sep = prefix_str.trim_end_matches('\\').trim_end_matches('/');
            let prefix_path = PathBuf::from(prefix_without_sep);
            if let Ok(rel) = path.strip_prefix(&prefix_path) {
                return Some(rel.to_path_buf());
            }
        }
    }

    None
}

/// Collect all command line arguments (git subcommand and its args)
fn collect_git_args() -> Vec<String> {
    env::args().skip(1).collect()
}

/// Find the real git executable (not this wrapper)
fn find_real_git() -> Option<PathBuf> {
    let current_exe = env::current_exe().ok()?;
    let wrapper_dir = current_exe.parent()?;

    // Search PATH for git executable
    let path_var = env::var_os("PATH")?;

    for dir in env::split_paths(&path_var) {
        // Skip the wrapper directory
        if dir == wrapper_dir {
            continue;
        }

        // Check for git.exe (Windows) or git (Unix)
        #[cfg(target_os = "windows")]
        let git_path = dir.join("git.exe");
        #[cfg(not(target_os = "windows"))]
        let git_path = dir.join("git");

        if git_path.exists() && git_path.is_file() {
            // Make sure it's not a symlink to our wrapper
            if let Ok(real_path) = std::fs::canonicalize(&git_path) {
                if let Ok(wrapper_real) = std::fs::canonicalize(&current_exe) {
                    if real_path != wrapper_real {
                        return Some(git_path);
                    }
                } else {
                    return Some(git_path);
                }
            } else {
                return Some(git_path);
            }
        }
    }

    // Fallback: try which/where
    #[cfg(target_os = "windows")]
    let output = Command::new("where")
        .arg("git.exe")
        .output()
        .ok()?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("which")
        .arg("git")
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let path = PathBuf::from(line.trim());
        if let Ok(real_path) = std::fs::canonicalize(&path) {
            if let Ok(wrapper_real) = std::fs::canonicalize(&current_exe) {
                if real_path != wrapper_real {
                    return Some(path);
                }
            } else {
                return Some(path);
            }
        }
    }

    None
}

/// Execute git in the specified directory with the given arguments
fn exec_git_in_dir(dir: &Path, args: &[String]) -> ! {
    let git_exe = find_real_git().unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        let default = PathBuf::from("git.exe");
        #[cfg(not(target_os = "windows"))]
        let default = PathBuf::from("git");
        default
    });

    let result = Command::new(&git_exe)
        .args(args)
        .current_dir(dir)
        .status();

    match result {
        Ok(status) => process::exit(status.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("git-wrapper: Failed to execute git: {}", e);
            process::exit(1);
        }
    }
}
