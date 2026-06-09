use clap::Parser;
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(name = "encoding-vfs", version, about = "Cross-platform encoding VFS: transparent source↔target conversion")]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Backend directory (where source-encoded files are stored)
    #[arg(short, long)]
    backend: PathBuf,

    /// Source encoding: "auto" for detection, or a specific encoding name (GBK, Shift_JIS, Big5...)
    #[arg(short = 's', long, default_value = "auto")]
    source_encoding: String,

    /// Target encoding: the encoding that mounted files will appear as
    #[arg(short = 't', long, default_value = "UTF-8")]
    target_encoding: String,

    /// Windows: Drive letter to mount (e.g., 'X')
    #[cfg(target_os = "windows")]
    #[arg(short = 'd', long, default_value = "X")]
    drive: char,

    /// Linux: Mount point path (e.g., '/mnt/gbk-vfs')
    #[cfg(target_os = "linux")]
    #[arg(short = 'm', long, default_value = "/mnt/gbk-vfs")]
    mount: String,

    /// Log level (trace/debug/info/warn/error)
    #[arg(short = 'L', long, default_value = "info")]
    log_level: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    let env_filter = format!("encoding_vfs={}", args.log_level);
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    info!("Starting Encoding VFS");
    info!("Backend: {:?}", args.backend);
    info!("Source encoding: {} | Target encoding: {}", args.source_encoding, args.target_encoding);

    // Load configuration, then merge CLI overrides
    let config = encoding_vfs_core::Config::load(args.config.as_ref())?;

    let mut encoding_config = config.encoding.clone();
    if args.source_encoding != "auto" {
        encoding_config.source_encoding = args.source_encoding.clone();
    }
    if args.target_encoding != "UTF-8" {
        encoding_config.target_encoding = args.target_encoding.clone();
    }

    // Create core VFS
    let vfs = encoding_vfs_core::EncodingVfs::new(&args.backend, encoding_config)?;

    #[cfg(target_os = "windows")]
    {
        info!("Platform: Windows");
        info!("Drive letter: {}", args.drive);

        let host = encoding_vfs_windows::WinFspVfsHost::new(vfs);
        encoding_vfs_windows::run(host, args.drive)?;
    }

    #[cfg(target_os = "linux")]
    {
        info!("Platform: Linux");
        info!("Mount point: {}", args.mount);

        let host = encoding_vfs_linux::FuseVfsHost::new(vfs);
        encoding_vfs_linux::run(host, &args.mount)?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = vfs;
        warn!("Unsupported platform: only Windows and Linux are supported");
    }

    Ok(())
}
