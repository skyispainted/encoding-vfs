use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use encoding_vfs_core::vfs::EncodingVfs;
use fuser::{
    FileType, FileAttr, Filesystem, MountOption, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr, Request,
};
use tracing::{debug, warn};

const TTL_SEC: f64 = 1.0;

fn make_attr(path: &Path, is_dir: bool, ino: u64) -> FileAttr {
    let full = path; // caller passes full path
    let (size, mtime, ctime, atime) = if is_dir {
        (0, SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH)
    } else {
        match std::fs::metadata(full) {
            Ok(m) => (
                m.len(),
                m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                m.created().unwrap_or(SystemTime::UNIX_EPOCH),
                m.accessed().unwrap_or(SystemTime::UNIX_EPOCH),
            ),
            Err(_) => (0, SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH),
        }
    };

    FileAttr {
        ino,
        size,
        blocks: (size + 511) / 512,
        atime,
        mtime,
        ctime,
        crtime: ctime,
        kind: if is_dir { FileType::Directory } else { FileType::RegularFile },
        perm: if is_dir { 0o755 } else { 0o644 },
        nlink: if is_dir { 2 } else { 1 },
        uid: unsafe { libc::getuid() },
        gid: unsafe { libc::getgid() },
        rdev: 0,
        blksize: 512,
    }
}

/// Shared state protected by a single mutex.
struct Inner {
    /// ino -> full backend path
    ino_map: HashMap<u64, PathBuf>,
    /// full backend path -> ino
    path_map: HashMap<PathBuf, u64>,
    next_ino: u64,
    /// fh -> (full_path, is_dir)
    handles: HashMap<u64, (PathBuf, bool)>,
    next_fh: u64,
}

impl Inner {
    fn new(root_ino: u64, root_path: PathBuf) -> Self {
        let mut ino_map = HashMap::new();
        let mut path_map = HashMap::new();
        ino_map.insert(root_ino, root_path.clone());
        path_map.insert(root_path, root_ino);
        Self {
            ino_map,
            path_map,
            next_ino: root_ino + 1,
            handles: HashMap::new(),
            next_fh: 1,
        }
    }

    fn lookup_ino(&self, ino: u64) -> Option<PathBuf> {
        self.ino_map.get(&ino).cloned()
    }

    fn lookup_path(&self, path: &Path) -> Option<u64> {
        self.path_map.get(path).copied()
    }

    fn insert(&mut self, path: PathBuf) -> u64 {
        if let Some(&ino) = self.path_map.get(&path) {
            return ino;
        }
        let ino = self.next_ino;
        self.next_ino += 1;
        self.ino_map.insert(ino, path.clone());
        self.path_map.insert(path, ino);
        ino
    }

    fn remove(&mut self, path: &Path) {
        if let Some(ino) = self.path_map.remove(path) {
            self.ino_map.remove(&ino);
        }
    }

    fn alloc_fh(&mut self, path: PathBuf, is_dir: bool) -> u64 {
        let fh = self.next_fh;
        self.next_fh += 1;
        self.handles.insert(fh, (path, is_dir));
        fh
    }

    fn free_fh(&mut self, fh: u64) {
        self.handles.remove(&fh);
    }

    fn get_fh(&self, fh: u64) -> Option<(PathBuf, bool)> {
        self.handles.get(&fh).map(|(p, d)| (p.clone(), *d))
    }
}

/// FUSE adapter that bridges fuser::Filesystem callbacks to EncodingVfs.
pub struct FuseVfsHost {
    vfs: EncodingVfs,
    inner: Mutex<Inner>,
}

impl FuseVfsHost {
    pub fn new(vfs: EncodingVfs) -> Self {
        let root = vfs.backend_dir.clone();
        let inner = Mutex::new(Inner::new(1, root));
        Self { vfs, inner }
    }
}

impl Filesystem for FuseVfsHost {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let mut inner = self.inner.lock().unwrap();
        let Some(parent_path) = inner.lookup_ino(parent) else {
            reply.error(libc::ENOENT);
            return;
        };

        let child_path = parent_path.join(name);
        if !child_path.exists() {
            reply.error(libc::ENOENT);
            return;
        }

        let is_dir = child_path.is_dir();
        let ino = inner.insert(child_path.clone());
        let attr = make_attr(&child_path, is_dir, ino);
        reply.entry(&TTL_SEC, &attr, 0);
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let inner = self.inner.lock().unwrap();
        let Some(path) = inner.lookup_ino(ino) else {
            reply.error(libc::ENOENT);
            return;
        };

        let is_dir = path.is_dir();
        let attr = make_attr(&path, is_dir, ino);
        reply.attr(&TTL_SEC, &attr);
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        let mut inner = self.inner.lock().unwrap();
        let Some(path) = inner.lookup_ino(ino) else {
            reply.error(libc::ENOENT);
            return;
        };

        if path.is_dir() {
            reply.error(libc::EISDIR);
            return;
        }

        let fh = inner.alloc_fh(path, false);
        reply.opened(fh, 0);
    }

    fn opendir(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        let mut inner = self.inner.lock().unwrap();
        let Some(path) = inner.lookup_ino(ino) else {
            reply.error(libc::ENOENT);
            return;
        };

        if !path.is_dir() {
            reply.error(libc::ENOTDIR);
            return;
        }

        let fh = inner.alloc_fh(path, true);
        reply.opened(fh, 0);
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let inner = self.inner.lock().unwrap();
        let Some((path, is_dir)) = inner.get_fh(fh) else {
            reply.error(libc::EBADF);
            return;
        };
        drop(inner);

        if is_dir {
            reply.error(libc::EISDIR);
            return;
        }

        let rel = path
            .strip_prefix(&self.vfs.backend_dir)
            .unwrap_or(&path);

        match self.vfs.read_file(rel, offset as u64, size as usize) {
            Ok(data) => reply.data(&data),
            Err(e) => {
                warn!(error = %e, path = ?rel, "read failed");
                reply.error(libc::EIO);
            }
        }
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        let inner = self.inner.lock().unwrap();
        let Some((path, is_dir)) = inner.get_fh(fh) else {
            reply.error(libc::EBADF);
            return;
        };
        drop(inner);

        if is_dir {
            reply.error(libc::EISDIR);
            return;
        }

        let rel = path
            .strip_prefix(&self.vfs.backend_dir)
            .unwrap_or(&path);

        match self.vfs.write_file(rel, offset as u64, data) {
            Ok(n) => reply.written(n as u32),
            Err(e) => {
                warn!(error = %e, path = ?rel, "write failed");
                reply.error(libc::EIO);
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let inner = self.inner.lock().unwrap();
        let Some(path) = inner.lookup_ino(ino) else {
            reply.error(libc::ENOENT);
            return;
        };
        drop(inner);

        let rel = path
            .strip_prefix(&self.vfs.backend_dir)
            .unwrap_or(&path);

        let entries = match self.vfs.read_dir(rel) {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, path = ?rel, "readdir failed");
                reply.error(libc::EIO);
                return;
            }
        };

        // "." entry
        if offset == 0 {
            if reply.add(ino, 1, FileType::Directory, ".") {
                reply.ok();
                return;
            }
        }

        // ".." entry
        if offset <= 1 {
            let parent_ino = if path == self.vfs.backend_dir {
                ino // root's .. is itself
            } else {
                let inner = self.inner.lock().unwrap();
                path.parent()
                    .and_then(|p| inner.lookup_path(p))
                    .unwrap_or(1)
            };
            if reply.add(parent_ino, 2, FileType::Directory, "..") {
                reply.ok();
                return;
            }
        }

        let mut actual_offset = offset.max(2);
        let mut inner = self.inner.lock().unwrap();
        for entry in entries.into_iter().skip((actual_offset - 2) as usize) {
            actual_offset += 1;
            let child_path = path.join(&entry.name);
            let child_ino = inner.insert(child_path.clone());
            let file_type = if entry.is_dir {
                FileType::Directory
            } else {
                FileType::RegularFile
            };
            let name = entry.name.to_string_lossy();
            if reply.add(child_ino, actual_offset, file_type, &name) {
                break;
            }
        }

        reply.ok();
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let Some(parent_path) = inner.lookup_ino(parent) else {
            reply.error(libc::ENOENT);
            return;
        };

        let child_path = parent_path.join(name);
        if let Some(p) = child_path.parent() {
            let _ = std::fs::create_dir_all(p);
        }

        match std::fs::File::create(&child_path) {
            Ok(_) => {
                let ino = inner.insert(child_path.clone());
                let fh = inner.alloc_fh(child_path.clone(), false);
                let attr = make_attr(&child_path, false, ino);
                reply.created(&TTL_SEC, &attr, 0, fh, 0);
            }
            Err(_) => reply.error(libc::EACCES),
        }
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let Some(parent_path) = inner.lookup_ino(parent) else {
            reply.error(libc::ENOENT);
            return;
        };

        let child_path = parent_path.join(name);
        match std::fs::create_dir_all(&child_path) {
            Ok(_) => {
                let ino = inner.insert(child_path.clone());
                let attr = make_attr(&child_path, true, ino);
                reply.entry(&TTL_SEC, &attr, 0);
            }
            Err(_) => reply.error(libc::EACCES),
        }
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let mut inner = self.inner.lock().unwrap();
        let Some(parent_path) = inner.lookup_ino(parent) else {
            reply.error(libc::ENOENT);
            return;
        };

        let child_path = parent_path.join(name);
        match std::fs::remove_file(&child_path) {
            Ok(_) => {
                inner.remove(&child_path);
                reply.ok();
            }
            Err(_) => reply.error(libc::ENOENT),
        }
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let mut inner = self.inner.lock().unwrap();
        let Some(parent_path) = inner.lookup_ino(parent) else {
            reply.error(libc::ENOENT);
            return;
        };

        let child_path = parent_path.join(name);
        match std::fs::remove_dir(&child_path) {
            Ok(_) => {
                inner.remove(&child_path);
                reply.ok();
            }
            Err(_) => reply.error(libc::ENOTEMPTY),
        }
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: ReplyEmpty,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let Some(from_parent) = inner.lookup_ino(parent) else {
            reply.error(libc::ENOENT);
            return;
        };
        let Some(_to_parent) = inner.lookup_ino(newparent) else {
            reply.error(libc::ENOENT);
            return;
        };

        let from = from_parent.join(name);
        let to = inner.lookup_ino(newparent).unwrap().join(newname);

        match std::fs::rename(&from, &to) {
            Ok(_) => {
                inner.remove(&from);
                inner.insert(to);
                reply.ok();
            }
            Err(_) => reply.error(libc::ENOENT),
        }
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        self.inner.lock().unwrap().free_fh(fh);
        reply.ok();
    }

    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        _flags: i32,
        reply: ReplyEmpty,
    ) {
        self.inner.lock().unwrap().free_fh(fh);
        reply.ok();
    }

    fn flush(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _name: &OsStr,
        _value: &[u8],
        _flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        reply.error(libc::ENOSYS);
    }

    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _name: &OsStr,
        _size: u32,
        reply: ReplyXattr,
    ) {
        reply.error(libc::ENOSYS);
    }

    fn listxattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _size: u32,
        reply: ReplyXattr,
    ) {
        reply.error(libc::ENOSYS);
    }

    fn removexattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _name: &OsStr,
        reply: ReplyEmpty,
    ) {
        reply.error(libc::ENOSYS);
    }

    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyStatfs) {
        reply.statfs(
            1_099_511_627_776, // total blocks
            549_755_813_888,   // free blocks
            549_755_813_888,   // available blocks
            0,                 // total inodes
            0,                 // free inodes
            512,               // bsize
            255,               // namemax
            0,                 // frsize
        );
    }
}

/// Start the FUSE virtual filesystem.
pub fn run(host: FuseVfsHost, mount_point: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mp = PathBuf::from(mount_point);

    if !mp.exists() {
        std::fs::create_dir_all(&mp)?;
    }

    debug!("Starting FUSE Encoding VFS on mount point: {}", mount_point);
    debug!("Backend directory: {:?}", host.vfs.backend_dir);
    debug!("Default encoding: {}", host.vfs.encoding_config.default_encoding);

    let options = vec![
        MountOption::FSName("EncodingVFS".to_string()),
        MountOption::AutoUnmount,
    ];

    fuser::mount2(host, &mp, &options)?;

    Ok(())
}
