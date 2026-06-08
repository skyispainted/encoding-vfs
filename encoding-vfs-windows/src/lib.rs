#[cfg(feature = "mount")]
pub mod winfsp_host;

#[cfg(feature = "mount")]
pub use winfsp_host::WinFspVfsHost;

#[cfg(feature = "mount")]
pub use winfsp_host::run;

#[cfg(not(feature = "mount"))]
/// Placeholder when WinFsp mount feature is not available.
/// Enable the `mount` feature and install WinFsp to use the virtual drive.
pub fn run_unavailable() {
    tracing::warn!("WinFsp mount support is disabled. Install WinFsp and enable the 'mount' feature.");
}
