//! Mount registry management for encoding-vfs.
//!
//! This module provides a unified way to track active VFS mounts across
//! Windows (drive letters) and Linux (mount point directories).
//! The registry is stored in `~/.encoding-vfs/mounts.json`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Information about a single active mount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    /// Mount point: "Y:" on Windows, "/mnt/vfs" on Linux
    pub mount_point: String,
    /// Source directory (backend) that is being mounted
    pub source: PathBuf,
    /// Process ID of the encoding-vfs process managing this mount
    pub pid: u32,
}

/// The mounts registry file structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MountsFile {
    mounts: Vec<MountInfo>,
}

/// Registry for managing active VFS mounts.
///
/// This struct handles reading/writing the mounts.json file and provides
/// methods to register, unregister, and query mounts.
#[derive(Debug, Default)]
pub struct MountsRegistry {
    mounts: Vec<MountInfo>,
}

impl MountsRegistry {
    /// Get the path to the mounts.json file.
    ///
    /// Location: `~/.encoding-vfs/mounts.json`
    fn mounts_file_path() -> Result<PathBuf, MountError> {
        let home = dirs::home_dir().ok_or_else(|| MountError::NoHomeDir)?;
        Ok(home.join(".encoding-vfs").join("mounts.json"))
    }

    /// Ensure the ~/.encoding-vfs directory exists.
    fn ensure_dir() -> Result<(), MountError> {
        let home = dirs::home_dir().ok_or_else(|| MountError::NoHomeDir)?;
        let dir = home.join(".encoding-vfs");
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| MountError::Io(e))?;
        }
        Ok(())
    }

    /// Load the registry from the mounts.json file.
    ///
    /// If the file doesn't exist, returns an empty registry.
    pub fn load() -> Result<Self, MountError> {
        let path = Self::mounts_file_path()?;

        if !path.exists() {
            return Ok(Self { mounts: Vec::new() });
        }

        let content = fs::read_to_string(&path).map_err(MountError::Io)?;
        let file: MountsFile = serde_json::from_str(&content).unwrap_or_default();

        Ok(Self {
            mounts: file.mounts,
        })
    }

    /// Save the registry to the mounts.json file.
    ///
    /// Uses atomic write (write to temp file, then rename) to prevent corruption.
    pub fn save(&self) -> Result<(), MountError> {
        Self::ensure_dir()?;
        let path = Self::mounts_file_path()?;

        let file = MountsFile {
            mounts: self.mounts.clone(),
        };
        let content = serde_json::to_string_pretty(&file).map_err(|e| MountError::Json(e))?;

        // Atomic write: write to temp file, then rename
        let temp_path = path.with_extension("json.tmp");
        let mut temp_file = fs::File::create(&temp_path).map_err(MountError::Io)?;
        temp_file
            .write_all(content.as_bytes())
            .map_err(MountError::Io)?;
        temp_file.flush().map_err(MountError::Io)?;
        drop(temp_file);

        // Rename temp to final (atomic on most filesystems)
        fs::rename(&temp_path, &path).map_err(MountError::Io)?;

        Ok(())
    }

    /// Register a new mount.
    pub fn register(&mut self, info: MountInfo) -> Result<(), MountError> {
        // Remove any existing entry for this mount point
        self.mounts.retain(|m| m.mount_point != info.mount_point);
        self.mounts.push(info);
        self.save()
    }

    /// Unregister a mount by its mount point.
    pub fn unregister(&mut self, mount_point: &str) -> Result<(), MountError> {
        self.mounts.retain(|m| m.mount_point != mount_point);
        self.save()
    }

    /// Find a mount by the given path.
    ///
    /// Returns the mount info if the path is within a mounted directory.
    /// For Windows: matches if the path's drive matches the mount point (e.g., "Y:")
    /// For Linux: matches if the path starts with the mount point
    pub fn find_by_path(&self, path: &Path) -> Option<&MountInfo> {
        let path_str = path.to_string_lossy();

        // Try exact match first, then prefix match
        // Sort by mount_point length (longest first) to match most specific
        let mut matches: Vec<&MountInfo> = self
            .mounts
            .iter()
            .filter(|m| {
                // Windows: check drive letter match (case-insensitive)
                if cfg!(windows) {
                    let mount_upper = m.mount_point.to_uppercase();
                    let path_upper = path_str.to_uppercase();
                    path_upper.starts_with(&mount_upper)
                } else {
                    // Linux: check path prefix
                    path_str.starts_with(&m.mount_point)
                }
            })
            .collect();

        // Sort by mount_point length descending (most specific match first)
        matches.sort_by(|a, b| b.mount_point.len().cmp(&a.mount_point.len()));

        matches.into_iter().next()
    }

    /// Clean up stale mount entries (where the process no longer exists).
    pub fn cleanup_stale(&mut self) {
        self.mounts.retain(|m| is_process_alive(m.pid));
    }

    /// Get all registered mounts.
    pub fn mounts(&self) -> &[MountInfo] {
        &self.mounts
    }
}

/// Check if a process with the given PID is still alive.
#[cfg(target_os = "windows")]
fn is_process_alive(pid: u32) -> bool {
    use std::process::Command;

    // Use tasklist to check if process exists
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains(&pid.to_string())
        }
        Err(_) => false,
    }
}

#[cfg(target_os = "linux")]
fn is_process_alive(pid: u32) -> bool {
    // On Linux, check if /proc/<pid> exists
    Path::new(&format!("/proc/{}", pid)).exists()
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn is_process_alive(_pid: u32) -> bool {
    // Unknown platform, assume alive
    true
}

/// Errors that can occur when working with the mount registry.
#[derive(Debug)]
pub enum MountError {
    /// Could not determine home directory
    NoHomeDir,
    /// I/O error
    Io(io::Error),
    /// JSON serialization/deserialization error
    Json(serde_json::Error),
}

impl std::fmt::Display for MountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MountError::NoHomeDir => write!(f, "Could not determine home directory"),
            MountError::Io(e) => write!(f, "I/O error: {}", e),
            MountError::Json(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for MountError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_info_serialization() {
        let info = MountInfo {
            mount_point: "Y:".to_string(),
            source: PathBuf::from("C:\\projects\\test"),
            pid: 12345,
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: MountInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.mount_point, info.mount_point);
        assert_eq!(parsed.source, info.source);
        assert_eq!(parsed.pid, info.pid);
    }

    #[test]
    fn test_find_by_path_windows() {
        let mut registry = MountsRegistry::default();
        registry.mounts.push(MountInfo {
            mount_point: "Y:".to_string(),
            source: PathBuf::from("C:\\projects\\test"),
            pid: 12345,
        });

        // Should find mount for Y: drive
        let result = registry.find_by_path(Path::new("Y:\\Some\\Path"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().source, PathBuf::from("C:\\projects\\test"));

        // Should not find mount for other drives
        let result = registry.find_by_path(Path::new("C:\\Users"));
        assert!(result.is_none());
    }

    #[test]
    fn test_find_by_path_linux() {
        let mut registry = MountsRegistry::default();
        registry.mounts.push(MountInfo {
            mount_point: "/mnt/vfs".to_string(),
            source: PathBuf::from("/home/user/project"),
            pid: 12345,
        });

        // Should find mount for paths under /mnt/vfs
        let result = registry.find_by_path(Path::new("/mnt/vfs/some/path"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().source, PathBuf::from("/home/user/project"));

        // Should not find mount for other paths
        let result = registry.find_by_path(Path::new("/home/other"));
        assert!(result.is_none());
    }
}
