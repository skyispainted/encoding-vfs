#[cfg(target_os = "linux")]
pub mod fuse_host;

#[cfg(target_os = "linux")]
pub use fuse_host::FuseVfsHost;

#[cfg(target_os = "linux")]
pub use fuse_host::run;
