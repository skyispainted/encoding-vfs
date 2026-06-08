#[cfg(feature = "mount")]
pub mod fuse_host;

#[cfg(feature = "mount")]
pub use fuse_host::FuseVfsHost;

#[cfg(feature = "mount")]
pub use fuse_host::run;
