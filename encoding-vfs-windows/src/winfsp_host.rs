use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use std::collections::HashMap;
use std::sync::Mutex;

use encoding_vfs_core::vfs::EncodingVfs;
use tracing::{debug, info, warn};
use winfsp::{
    U16CStr, Result,
    filesystem::{
        DirInfo, DirMarker, FileInfo, FileSystemContext, FileSecurity, OpenFileInfo, VolumeInfo,
        WideNameInfo,
    },
    host::{FileSystemHost, FileSystemParams, OperationGuardStrategy, VolumeParams},
};

/// File context that stores the path and whether it's a directory.
pub struct FileContext {
    path: PathBuf,
    is_dir: bool,
}

/// Global write buffer cache for handling chunked writes.
/// Key: file path, Value: accumulated write data
static WRITE_BUFFERS: std::sync::LazyLock<Mutex<HashMap<PathBuf, Vec<u8>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Global read cache for storing fully-converted file content.
/// Key: file path, Value: (converted UTF-8 content, backend modified time)
///
/// This is essential for correct encoding conversion. The `read` callback receives
/// byte offsets in the converted (UTF-8) content, but the backend file is in a
/// different encoding (e.g., GBK). Reading the backend at arbitrary byte offsets
/// and converting each chunk independently can split multi-byte characters at chunk
/// boundaries, producing garbled output. By caching the fully-converted content,
/// we ensure correct conversion regardless of chunk boundaries.
static READ_CACHE: std::sync::LazyLock<Mutex<HashMap<PathBuf, (Vec<u8>, SystemTime)>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// WinFsp adapter that implements the virtual filesystem operations.
pub struct WinFspVfsHost {
    vfs: EncodingVfs,
}

impl WinFspVfsHost {
    pub fn new(vfs: EncodingVfs) -> Self {
        Self { vfs }
    }

    fn rel_path<'a>(&self, full: &'a PathBuf) -> &'a std::path::Path {
        full.strip_prefix(&self.vfs.backend_dir).unwrap_or(full)
    }

    fn resolve_path(&self, name: &str) -> PathBuf {
        let trimmed = name.trim_start_matches('\\');
        if trimmed.is_empty() {
            self.vfs.backend_dir.clone()
        } else {
            self.vfs.backend_dir.join(trimmed)
        }
    }

    fn file_time(system_time: SystemTime) -> u64 {
        system_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }

    fn file_attributes(is_dir: bool) -> u32 {
        if is_dir { 0x10 } else { 0x80 }
    }

    fn fill_file_info(fi: &mut FileInfo, is_dir: bool, size: u64, modified: SystemTime) {
        let mtime = Self::file_time(modified);
        fi.file_attributes = Self::file_attributes(is_dir);
        fi.file_size = size;
        fi.allocation_size = size;
        fi.last_write_time = mtime;
        fi.last_access_time = mtime;
        fi.creation_time = mtime;
        fi.change_time = mtime;
        fi.index_number = 0;
        fi.hard_links = 0;
        fi.reparse_tag = 0;
    }

    /// Get converted file size (target encoding) for a file
    fn get_converted_size(&self, full_path: &Path, rel_path: &Path) -> u64 {
        match self.vfs.get_file_info(rel_path) {
            Ok(info) => info.size,
            Err(_) => std::fs::metadata(full_path).map(|m| m.len()).unwrap_or(0),
        }
    }
}

impl FileSystemContext for WinFspVfsHost {
    type FileContext = Arc<FileContext>;

    fn get_security_by_name(
        &self,
        file_name: &U16CStr,
        security_descriptor: Option<&mut [c_void]>,
        _reparse_point_resolver: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
    ) -> Result<FileSecurity> {
        let name = file_name.to_string_lossy();
        let full_path = self.resolve_path(&name);

        if !full_path.exists() {
            return Err(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND.into());
        }

        let is_dir = full_path.is_dir();
        let attrs = Self::file_attributes(is_dir);

        if name.contains("KItem.cpp") {
            debug!("get_security_by_name: name={} full_path={} is_dir={} attrs=0x{:X}",
                name, full_path.display(), is_dir, attrs);
        }

        // Return no security descriptor; let WinFsp use defaults.
        Ok(FileSecurity {
            reparse: false,
            sz_security_descriptor: 0,
            attributes: attrs,
        })
    }

    fn open(
        &self,
        file_name: &U16CStr,
        create_options: u32,
        granted_access: u32,
        file_info: &mut OpenFileInfo,
    ) -> Result<Self::FileContext> {
        const FILE_DIRECTORY_FILE: u32 = 0x00000001;
        const FILE_WRITE_DATA: u32 = 0x00000002;
        const FILE_APPEND_DATA: u32 = 0x00000004;
        const GENERIC_WRITE: u32 = 0x40000000;

        let name = file_name.to_string_lossy();
        let full_path = self.resolve_path(&name);

        if !full_path.exists() {
            return Err(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND.into());
        }

        let is_dir = full_path.is_dir();
        let is_dir_requested = create_options & FILE_DIRECTORY_FILE != 0;

        if name.contains("KItem.cpp") {
            debug!("open: name={} full_path={} is_dir={} is_dir_requested={} create_options=0x{:X}",
                name, full_path.display(), is_dir, is_dir_requested, create_options);
        }

        // Only reject if a directory was explicitly requested but the target is not one.
        // Allow opening a directory without FILE_DIRECTORY_FILE (common with
        // FILE_FLAG_BACKUP_SEMANTICS for handle-based operations).
        if is_dir_requested && !is_dir {
            return Err(windows::Win32::Foundation::STATUS_NOT_A_DIRECTORY.into());
        }

        // If opening with write access, clear any stale buffer for this path.
        // This prevents data from a previous handle's failed/crashed write from
        // being flushed when this new handle is closed.
        let is_write_open = granted_access & (FILE_WRITE_DATA | FILE_APPEND_DATA | GENERIC_WRITE) != 0;
        if is_write_open && !is_dir {
            let mut buffers = WRITE_BUFFERS.lock().unwrap();
            if buffers.remove(&full_path).is_some() {
                info!(path = ?self.rel_path(&full_path), "cleared stale write buffer on open");
            }
        }

        let metadata = std::fs::metadata(&full_path)
            .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        // Use converted file size for text files
        let size = if is_dir {
            0
        } else {
            let rel = self.rel_path(&full_path);
            self.get_converted_size(&full_path, rel)
        };

        Self::fill_file_info(file_info.as_mut(), is_dir, size, modified);

        Ok(Arc::new(FileContext {
            path: full_path,
            is_dir,
        }))
    }

    fn close(&self, context: Self::FileContext) {
        // No-op: cleanup already handles flushing
    }

    fn create(
        &self,
        file_name: &U16CStr,
        create_options: u32,
        _granted_access: u32,
        file_attributes: u32,
        _security_descriptor: Option<&[c_void]>,
        _allocation_size: u64,
        _extra_buffer: Option<&[u8]>,
        _extra_buffer_is_reparse_point: bool,
        file_info: &mut OpenFileInfo,
    ) -> Result<Self::FileContext> {
        const FILE_DIRECTORY_FILE: u32 = 0x00000001;
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x00000010;

        let full_path = self.resolve_path(&file_name.to_string_lossy());
        // Determine directory status from both create_options and file_attributes.
        let is_dir = create_options & FILE_DIRECTORY_FILE != 0
            || file_attributes & FILE_ATTRIBUTE_DIRECTORY != 0;

        let fname = file_name.to_string_lossy();
        if fname.contains("KItem") || fname.contains("tmp") || fname.contains("vscode") {
            debug!("create: name={} full_path={} is_dir={} create_options=0x{:X} file_attributes=0x{:X}",
                fname, full_path.display(), is_dir, create_options, file_attributes);
        }

        if is_dir {
            std::fs::create_dir_all(&full_path)
                .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
        } else {
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
            }
            std::fs::File::create(&full_path)
                .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
        }

        Self::fill_file_info(file_info.as_mut(), is_dir, 0, SystemTime::now());

        Ok(Arc::new(FileContext {
            path: full_path.clone(),
            is_dir,
        }))
    }

    fn cleanup(&self, context: &Self::FileContext, _file_name: Option<&U16CStr>, _flags: u32) {
        info!(path = ?context.path, "cleanup called");
        // Flush any pending writes when the user-mode handle is closed
        let mut buffers = WRITE_BUFFERS.lock().unwrap();
        if let Some(buffer) = buffers.remove(&context.path) {
            if !buffer.is_empty() {
                let rel = self.rel_path(&context.path);
                info!(path = ?rel, bytes = buffer.len(), "flushing write buffer on cleanup");
                match self.vfs.write_file(rel, 0, &buffer) {
                    Ok(_) => info!(path = ?rel, bytes = buffer.len(), "flushed write buffer on cleanup"),
                    Err(e) => warn!(path = ?rel, error = %e, "failed to flush write buffer on cleanup"),
                }
                // Invalidate read cache so subsequent reads get the updated content
                READ_CACHE.lock().unwrap().remove(&context.path);
            }
        }
    }

    fn overwrite(
        &self,
        context: &Self::FileContext,
        _file_attributes: u32,
        _replace_file_attributes: bool,
        _allocation_size: u64,
        _extra_buffer: Option<&[u8]>,
        file_info: &mut FileInfo,
    ) -> Result<()> {
        if context.is_dir {
            return Err(windows::Win32::Foundation::STATUS_FILE_IS_A_DIRECTORY.into());
        }
        // Truncate the file to zero
        let rel = self.rel_path(&context.path);
        self.vfs.truncate_backend(&context.path)
            .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
        // Invalidate read cache since backend content changed
        READ_CACHE.lock().unwrap().remove(&context.path);
        let metadata = std::fs::metadata(&context.path)
            .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
        let modified = metadata.modified().unwrap_or(SystemTime::now());
        let size = self.get_converted_size(&context.path, rel);
        Self::fill_file_info(file_info, false, size, modified);
        Ok(())
    }

    fn set_file_size(
        &self,
        context: &Self::FileContext,
        new_size: u64,
        _set_allocation_size: bool,
        file_info: &mut FileInfo,
    ) -> Result<()> {
        if context.is_dir {
            return Err(windows::Win32::Foundation::STATUS_FILE_IS_A_DIRECTORY.into());
        }

        let rel = self.rel_path(&context.path);
        info!(path = ?rel, new_size = new_size, "set_file_size called");

        // Clear/truncate any pending write buffer to stay consistent
        {
            let mut buffers = WRITE_BUFFERS.lock().unwrap();
            if let Some(buffer) = buffers.get_mut(&context.path) {
                if new_size == 0 {
                    buffer.clear();
                } else if (new_size as usize) < buffer.len() {
                    buffer.truncate(new_size as usize);
                }
            }
        }

        if new_size == 0 {
            // Fast path: truncate backend directly
            self.vfs.truncate_backend(&context.path)
                .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
            // Invalidate read cache since backend content changed
            READ_CACHE.lock().unwrap().remove(&context.path);
        } else {
            // Read current content, truncate/pad, and write back
            let backend_size = std::fs::metadata(&context.path)
                .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?
                .len() as usize;

            let current_utf8 = match self.vfs.read_file(rel, 0, backend_size) {
                Ok(content) => content,
                Err(_) => {
                    let metadata = std::fs::metadata(&context.path)
                        .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
                    let modified = metadata.modified().unwrap_or(SystemTime::now());
                    let size = self.get_converted_size(&context.path, rel);
                    Self::fill_file_info(file_info, false, size, modified);
                    return Ok(());
                }
            };

            let new_utf8 = if (new_size as usize) < current_utf8.len() {
                current_utf8[..new_size as usize].to_vec()
            } else if (new_size as usize) > current_utf8.len() {
                let mut padded = current_utf8;
                padded.resize(new_size as usize, b' ');
                padded
            } else {
                current_utf8
            };

            // Truncate backend first, then write new content
            self.vfs.truncate_backend(&context.path)
                .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
            if !new_utf8.is_empty() {
                match self.vfs.write_file(rel, 0, &new_utf8) {
                    Ok(_) => {},
                    Err(e) => warn!(error = %e, "set_file_size: write_file failed"),
                }
            }
            // Invalidate read cache since backend content changed
            READ_CACHE.lock().unwrap().remove(&context.path);
        }

        let modified = std::fs::metadata(&context.path)
            .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?
            .modified()
            .unwrap_or(SystemTime::now());
        let size = self.get_converted_size(&context.path, rel);
        Self::fill_file_info(file_info, false, size, modified);
        Ok(())
    }

    fn set_basic_info(
        &self,
        context: &Self::FileContext,
        file_attributes: u32,
        creation_time: u64,
        last_access_time: u64,
        last_write_time: u64,
        last_change_time: u64,
        file_info: &mut FileInfo,
    ) -> Result<()> {
        let _ = (file_attributes, creation_time, last_access_time, last_write_time, last_change_time);
        let metadata = std::fs::metadata(&context.path)
            .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let rel = self.rel_path(&context.path);
        let size = self.get_converted_size(&context.path, rel);
        Self::fill_file_info(file_info, context.is_dir, size, modified);
        Ok(())
    }

    fn flush(&self, context: Option<&Self::FileContext>, file_info: &mut FileInfo) -> Result<()> {
        let Some(ctx) = context else {
            return Ok(());
        };
        let metadata = std::fs::metadata(&ctx.path)
            .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let rel = self.rel_path(&ctx.path);
        let size = self.get_converted_size(&ctx.path, rel);
        Self::fill_file_info(file_info, ctx.is_dir, size, modified);
        Ok(())
    }

    fn get_file_info(&self, context: &Self::FileContext, file_info: &mut FileInfo) -> Result<()> {
        if context.path.to_string_lossy().contains("KItem.cpp") {
            debug!("get_file_info: path={} is_dir={}", context.path.display(), context.is_dir);
        }
        if context.is_dir {
            Self::fill_file_info(file_info, true, 0, SystemTime::now());
        } else {
            let metadata = std::fs::metadata(&context.path)
                .map_err(|_| windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND)?;
            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let rel = self.rel_path(&context.path);
            let size = self.get_converted_size(&context.path, rel);
            Self::fill_file_info(file_info, false, size, modified);
        }
        Ok(())
    }

    fn get_security(
        &self,
        _context: &Self::FileContext,
        _security_descriptor: Option<&mut [c_void]>,
    ) -> Result<u64> {
        // Return no security descriptor; let WinFsp use defaults.
        Ok(0)
    }

    fn read(&self, context: &Self::FileContext, buffer: &mut [u8], offset: u64) -> Result<u32> {
        if context.is_dir {
            return Err(windows::Win32::Foundation::STATUS_FILE_IS_A_DIRECTORY.into());
        }

        let rel = self.rel_path(&context.path);

        // Use read cache to serve chunks from fully-converted content.
        // This avoids the critical bug where reading the backend file at arbitrary
        // byte offsets and converting each chunk independently splits multi-byte
        // characters at chunk boundaries, producing garbled UTF-8 output.
        let converted = {
            let mut cache = READ_CACHE.lock().unwrap();
            let mtime = std::fs::metadata(&context.path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            let need_refresh = cache.get(&context.path)
                .map(|(_, cached_mtime)| *cached_mtime != mtime)
                .unwrap_or(true);

            if need_refresh {
                // Read ENTIRE backend file and convert as a whole unit.
                // This is the ONLY correct way to handle encoding conversion
                // for variable-width encodings like GBK.
                let backend_size = std::fs::metadata(&context.path)
                    .map(|m| m.len())
                    .unwrap_or(0) as usize;
                match self.vfs.read_file(rel, 0, backend_size.max(1)) {
                    Ok(full_content) => {
                        cache.insert(context.path.clone(), (full_content, mtime));
                    }
                    Err(e) => {
                        warn!(error = %e, path = ?rel, "read: failed to read full content for cache");
                        return Err(windows::Win32::Foundation::STATUS_ACCESS_DENIED.into());
                    }
                }
            }

            cache.get(&context.path).unwrap().0.clone()
        };

        // Serve the requested chunk from the cached converted content
        let offset = offset as usize;
        if offset >= converted.len() {
            return Ok(0);
        }
        let available = &converted[offset..];
        let len = available.len().min(buffer.len());
        buffer[..len].copy_from_slice(&available[..len]);
        Ok(len as u32)
    }

    fn write(
        &self,
        context: &Self::FileContext,
        buffer: &[u8],
        offset: u64,
        _write_to_eof: bool,
        _constrained_io: bool,
        file_info: &mut FileInfo,
    ) -> Result<u32> {
        if context.is_dir {
            return Err(windows::Win32::Foundation::STATUS_FILE_IS_A_DIRECTORY.into());
        }

        // Buffer ALL writes regardless of offset to handle chunked writes correctly.
        // WinFsp can split a single write into multiple chunks with different offsets.
        // We accumulate all chunks in memory, then flush the complete buffer on close/rename.
        // This avoids encoding issues when WinFsp splits writes across multi-byte boundaries,
        // and avoids the bug where offset>0 writes on a new file produce space-padded garbage.

        let mut buffers = WRITE_BUFFERS.lock().unwrap();
        let file_buffer = buffers.entry(context.path.clone()).or_insert_with(Vec::new);

        if offset == 0 {
            // First write (or full rewrite): replace buffer content
            file_buffer.clear();
            file_buffer.extend_from_slice(buffer);
        } else if offset as usize <= file_buffer.len() {
            // Overlap with existing buffer: overwrite the overlapping part
            let start = offset as usize;
            let end = start + buffer.len();
            if end > file_buffer.len() {
                file_buffer.resize(end, 0);
            }
            file_buffer[start..end].copy_from_slice(buffer);
        } else {
            // Gap between buffer end and this write: pad with zeros
            file_buffer.resize(offset as usize, 0);
            file_buffer.extend_from_slice(buffer);
        }

        info!(
            path = ?self.rel_path(&context.path),
            chunk_offset = offset,
            chunk_size = buffer.len(),
            total_buffered = file_buffer.len(),
            "buffered write chunk"
        );

        // Update file info with the buffered size (as UTF-8)
        let size = file_buffer.len() as u64;
        let modified = SystemTime::now();
        Self::fill_file_info(file_info, false, size, modified);

        Ok(buffer.len() as u32)
    }

    fn read_directory(
        &self,
        context: &Self::FileContext,
        _pattern: Option<&U16CStr>,
        marker: DirMarker,
        buffer: &mut [u8],
    ) -> Result<u32> {
        if !context.is_dir {
            return Err(windows::Win32::Foundation::STATUS_NOT_A_DIRECTORY.into());
        }

        let rel = self.rel_path(&context.path);
        let entries = self
            .vfs
            .read_dir(rel)
            .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;

        let mut cursor: u32 = 0;

        let mut add_entry = |name: &str, is_dir: bool, size: u64, mtime: u64| -> Option<()> {
            let mut di = DirInfo::<255>::new();
            di.file_info_mut().file_attributes = Self::file_attributes(is_dir);
            di.file_info_mut().file_size = size;
            di.file_info_mut().allocation_size = size;
            di.file_info_mut().last_write_time = mtime;
            di.file_info_mut().last_access_time = mtime;
            di.file_info_mut().creation_time = mtime;
            di.file_info_mut().change_time = mtime;
            di.set_name(name).ok()?;
            di.append_to_buffer(buffer, &mut cursor).then_some(())
        };

        // On first call (marker is None), prepend "." and ".."
        if marker.is_none() {
            let epoch = Self::file_time(SystemTime::UNIX_EPOCH);
            add_entry(".", true, 0, epoch);
            add_entry("..", true, 0, epoch);
        }

        // Skip entries before the marker position
        let skip_past: Option<String> = marker.inner_as_cstr().map(|c| c.to_string_lossy());
        let mut skipping = skip_past.is_some();

        for entry in entries {
            if skipping {
                let entry_name = entry.name.to_string_lossy();
                if entry_name.as_ref() == skip_past.as_deref().unwrap() {
                    skipping = false;
                }
                continue;
            }
            let entry_name = entry.name.to_string_lossy();
            if entry_name == "KItem.cpp" {
                debug!("read_dir entry: name={} is_dir={} size={} mtime={:?}",
                    entry_name, entry.is_dir, entry.size, entry.modified);
            }
            let mtime = Self::file_time(entry.modified);
            if add_entry(&entry_name, entry.is_dir, entry.size, mtime).is_none() {
                break;
            }
        }

        DirInfo::<255>::finalize_buffer(buffer, &mut cursor);
        Ok(cursor)
    }

    fn rename(
        &self,
        _context: &Self::FileContext,
        file_name: &U16CStr,
        new_file_name: &U16CStr,
        _replace_if_exists: bool,
    ) -> Result<()> {
        let from = self.resolve_path(&file_name.to_string_lossy());
        let to = self.resolve_path(&new_file_name.to_string_lossy());

        if !from.exists() {
            return Err(windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND.into());
        }

        // Flush any pending writes for the source file before rename
        {
            let mut buffers = WRITE_BUFFERS.lock().unwrap();
            if let Some(buffer) = buffers.remove(&from) {
                if !buffer.is_empty() {
                    let rel = self.rel_path(&from);
                    info!(path = ?rel, bytes = buffer.len(), "flushing write buffer before rename");
                    match self.vfs.write_file(rel, 0, &buffer) {
                        Ok(_) => info!(path = ?rel, bytes = buffer.len(), "flushed write buffer before rename"),
                        Err(e) => warn!(path = ?rel, error = %e, "failed to flush write buffer before rename"),
                    }
                    // Invalidate read cache
                    READ_CACHE.lock().unwrap().remove(&from);
                }
            }
        }

        std::fs::rename(&from, &to)
            .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED.into())
    }

    fn set_delete(
        &self,
        context: &Self::FileContext,
        _file_name: &U16CStr,
        delete_file: bool,
    ) -> Result<()> {
        if delete_file {
            if context.is_dir {
                std::fs::remove_dir_all(&context.path)
                    .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
            } else {
                std::fs::remove_file(&context.path)
                    .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
            }
        }
        Ok(())
    }

    fn get_volume_info(&self, out_volume_info: &mut VolumeInfo) -> Result<()> {
        debug!("get_volume_info called");
        out_volume_info.total_size = 1_099_511_627_776;
        out_volume_info.free_size = 549_755_813_888;
        out_volume_info.set_volume_label("EncodingVFS");
        debug!("get_volume_info -> ok");
        Ok(())
    }
}

/// Start the WinFsp virtual filesystem.
pub fn run(host: WinFspVfsHost, drive_letter: char) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use encoding_vfs_core::{MountsRegistry, MountInfo};

    info!("Starting WinFsp Encoding VFS on drive {}:", drive_letter);
    info!("Backend directory: {:?}", host.vfs.backend_dir);
    info!("Default encoding: {}", host.vfs.encoding_config.default_encoding);

    // Save backend_dir before host is moved
    let backend_dir = host.vfs.backend_dir.clone();

    // Load mounts registry and cleanup stale entries before mounting
    let mut registry = MountsRegistry::load().unwrap_or_else(|e| {
        warn!("Failed to load mounts registry: {}, starting fresh", e);
        MountsRegistry::default()
    });
    registry.cleanup_stale();

    let mut vp = VolumeParams::new();
    vp.sector_size(512);
    vp.sectors_per_allocation_unit(8);
    vp.case_sensitive_search(false);
    vp.case_preserved_names(true);
    vp.unicode_on_disk(true);
    vp.volume_info_timeout(1000);
    vp.filesystem_name("EncodingVFS");

    let options = FileSystemParams {
        use_dir_info_by_name: false,
        volume_params: vp,
        guard_strategy: OperationGuardStrategy::Coarse,
        debug_mode: Default::default(),
    };

    let mut fs = FileSystemHost::new_with_options(options, host)?;

    let mount_str = format!("{}:", drive_letter);
    fs.mount(&mount_str)?;
    fs.start()?;

    // Register mount in mounts.json
    if let Err(e) = registry.register(MountInfo {
        mount_point: mount_str.clone(),
        source: backend_dir,
        pid: std::process::id(),
    }) {
        warn!("Failed to register mount: {}", e);
    } else {
        info!("Registered mount in mounts.json");
    }

    info!("Encoding VFS mounted on {}:", drive_letter);
    info!("Press Ctrl+C to unmount and exit.");

    static SHUTDOWN: AtomicBool = AtomicBool::new(false);

    unsafe extern "system" fn console_handler(ctrl_type: u32) -> i32 {
        if ctrl_type == 0 || ctrl_type == 2 {
            SHUTDOWN.store(true, Ordering::SeqCst);
            1
        } else {
            0
        }
    }

    unsafe {
        let handler: unsafe extern "system" fn(u32) -> i32 = console_handler;
        windows::Win32::System::Console::SetConsoleCtrlHandler(
            Some(std::mem::transmute(handler)),
            true,
        )?;
    }

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if SHUTDOWN.load(Ordering::SeqCst) {
            info!("Shutdown signal received");
            break;
        }
    }

    fs.unmount();
    fs.stop();

    // Unregister mount from mounts.json
    if let Err(e) = registry.unregister(&mount_str) {
        warn!("Failed to unregister mount: {}", e);
    } else {
        info!("Unregistered mount from mounts.json");
    }

    info!("Encoding VFS unmounted from {}:", drive_letter);
    Ok(())
}
