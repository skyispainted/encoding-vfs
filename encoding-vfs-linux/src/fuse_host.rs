use encoding_vfs_core::vfs::{EncodingVfs, DirEntry, FileInfo};
use encoding_vfs_core::error::VfsError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{info, warn};

/// State for an open file handle
struct OpenFile {
    path: PathBuf,
    is_dir: bool,
    dir_entries: Option<Vec<DirEntry>>,
    dir_index: usize,
}

/// FUSE adapter that implements the virtual filesystem operations.
/// This bridges between FUSE callbacks and the core EncodingVfs.
pub struct FuseVfsHost {
    vfs: EncodingVfs,
    open_files: HashMap<u64, OpenFile>,
    next_fh: u64,
    /// Inode counter
    next_ino: u64,
    /// Root inode is always 1
    root_ino: u64,
}

impl FuseVfsHost {
    pub fn new(vfs: EncodingVfs) -> Self {
        Self {
            vfs,
            open_files: HashMap::new(),
            next_fh: 1,
            next_ino: 2, // Root is 1
            root_ino: 1,
        }
    }

    fn alloc_fh(&mut self, path: PathBuf, is_dir: bool) -> u64 {
        let fh = self.next_fh;
        self.next_fh += 1;
        self.open_files.insert(fh, OpenFile {
            path,
            is_dir,
            dir_entries: None,
            dir_index: 0,
        });
        fh
    }

    fn free_fh(&mut self, fh: u64) {
        self.open_files.remove(&fh);
    }

    fn alloc_ino(&mut self) -> u64 {
        let ino = self.next_ino;
        self.next_ino += 1;
        ino
    }

    /// Convert SystemTime to UNIX timestamp
    fn time_to_unix(secs: SystemTime) -> i64 {
        secs.duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// Core read operation - returns UTF-8 data from GBK backend
    pub fn read(&self, fh: u64, offset: u64, len: usize) -> Result<Vec<u8>, VfsError> {
        let file = self.open_files.get(&fh)
            .ok_or_else(|| VfsError::NotFound(format!("file handle {}", fh).into()))?;

        let rel_path = file.path.strip_prefix(&self.vfs.backend_dir)
            .unwrap_or(&file.path);

        self.vfs.read_file(rel_path, offset, len)
    }

    /// Core write operation - converts UTF-8 to GBK before writing
    pub fn write(&mut self, fh: u64, offset: u64, data: &[u8]) -> Result<u64, VfsError> {
        let file = self.open_files.get(&fh)
            .ok_or_else(|| VfsError::NotFound(format!("file handle {}", fh).into()))?;

        let rel_path = file.path.strip_prefix(&self.vfs.backend_dir)
            .unwrap_or(&file.path);

        self.vfs.write_file(rel_path, offset, data)
    }

    /// Get file info with UTF-8 size
    pub fn get_file_info(&self, fh: u64) -> Result<FileInfo, VfsError> {
        let file = self.open_files.get(&fh)
            .ok_or_else(|| VfsError::NotFound(format!("file handle {}", fh).into()))?;

        let rel_path = file.path.strip_prefix(&self.vfs.backend_dir)
            .unwrap_or(&file.path);

        self.vfs.get_file_info(rel_path)
    }
}

/// Start the FUSE virtual filesystem.
/// Mounts the encoding VFS on the specified mount point.
pub fn run(host: FuseVfsHost, mount_point: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting FUSE Encoding VFS on mount point: {}", mount_point);

    // TODO: Implement actual fuser::Filesystem trait
    // This requires the fuser crate's mount2 function and Filesystem trait

    // For now, this is a placeholder that logs the configuration
    info!("Backend directory: {:?}", host.vfs.backend_dir);
    info!("Default encoding: {}", host.vfs.encoding_config.default_encoding);
    info!("Auto detect: {}", host.vfs.encoding_config.auto_detect);
    info!("Cache max entries: {}", host.vfs.encoding_config.cache_max_entries);

    // In full implementation, we would:
    // 1. Implement fuser::Filesystem trait for FuseVfsHost
    // 2. Call fuser::mount2() to mount the filesystem
    // 3. Block until unmount

    warn!("FUSE implementation is placeholder - full implementation requires fuser crate");

    Ok(())
}
