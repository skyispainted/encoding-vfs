use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

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
        _granted_access: u32,
        file_info: &mut OpenFileInfo,
    ) -> Result<Self::FileContext> {
        const FILE_DIRECTORY_FILE: u32 = 0x00000001;

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

    fn close(&self, _context: Self::FileContext) {}

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

    fn cleanup(&self, _context: &Self::FileContext, _file_name: Option<&U16CStr>, _flags: u32) {}

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
        match self.vfs.write_file(rel, 0, &[]) {
            Ok(_) => {}
            Err(_) => {
                // Fallback: try truncating via std
                std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&context.path)
                    .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
            }
        }
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
        _new_size: u64,
        _set_allocation_size: bool,
        file_info: &mut FileInfo,
    ) -> Result<()> {
        if context.is_dir {
            return Err(windows::Win32::Foundation::STATUS_FILE_IS_A_DIRECTORY.into());
        }
        // Don't truncate here - the actual size change happens in write().
        // The requested size is in target encoding (UTF-8), which doesn't match
        // the backend file's encoded size. Truncating here causes stale data at tail.
        let metadata = std::fs::metadata(&context.path)
            .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
        let modified = metadata.modified().unwrap_or(SystemTime::now());
        let rel = self.rel_path(&context.path);
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
        match self.vfs.read_file(rel, offset, buffer.len()) {
            Ok(data) => {
                let len = data.len().min(buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len as u32)
            }
            Err(e) => {
                warn!(error = %e, path = ?rel, "read failed");
                Err(windows::Win32::Foundation::STATUS_ACCESS_DENIED.into())
            }
        }
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

        let rel = self.rel_path(&context.path);
        match self.vfs.write_file(rel, offset, buffer) {
            Ok(_) => {
                // Return buffer.len() to WinFsp: this is the number of bytes it asked
                // us to write. The actual encoded byte count may differ (UTF-8 vs GBK),
                // but WinFsp expects this value to equal buffer.len() or it will retry
                // writes at wrong offsets, causing duplicate/trailing content.
                let metadata = std::fs::metadata(&context.path)
                    .map_err(|_| windows::Win32::Foundation::STATUS_ACCESS_DENIED)?;
                let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let size = self.get_converted_size(&context.path, rel);
                Self::fill_file_info(file_info, false, size, modified);
                Ok(buffer.len() as u32)
            }
            Err(e) => {
                warn!(error = %e, path = ?rel, "write failed");
                Err(windows::Win32::Foundation::STATUS_ACCESS_DENIED.into())
            }
        }
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
