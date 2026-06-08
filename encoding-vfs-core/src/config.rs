use serde::Deserialize;
use std::path::PathBuf;
use crate::error::VfsError;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub backend: BackendConfig,
    pub mount: MountConfig,
    pub encoding: EncodingConfig,
    pub log: Option<LogConfig>,
    pub filter: Option<FilterConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackendConfig {
    pub backend_dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MountConfig {
    /// Windows: drive letter (e.g., 'X')
    pub drive_letter: Option<char>,
    /// Linux: mount point path (e.g., "/mnt/gbk-vfs")
    pub mount_point: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EncodingConfig {
    /// Source encoding: "auto" for auto-detection, or a specific encoding name like "GBK", "Shift_JIS".
    #[serde(default = "default_source_encoding")]
    pub source_encoding: String,
    /// Target encoding: the encoding that files will appear as when mounted. Usually "UTF-8".
    #[serde(default = "default_target_encoding")]
    pub target_encoding: String,
    /// Fallback encoding when auto-detection fails. Only used when source_encoding is "auto".
    #[serde(default = "default_encoding")]
    pub default_encoding: String,
    /// Deprecated: use source_encoding = "auto" instead. Kept for backward compatibility.
    #[serde(default = "default_true")]
    pub auto_detect: bool,
    #[serde(default = "default_sample_bytes")]
    pub detect_sample_bytes: usize,
    #[serde(default = "default_cache_max_entries")]
    pub cache_max_entries: u64,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
}

fn default_source_encoding() -> String {
    "auto".to_string()
}

fn default_target_encoding() -> String {
    "UTF-8".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_file")]
    pub file: Option<PathBuf>,
}

fn default_encoding() -> String {
    "GBK".to_string()
}

fn default_true() -> bool {
    true
}

fn default_sample_bytes() -> usize {
    8192
}

fn default_cache_max_entries() -> u64 {
    10000
}

fn default_cache_ttl() -> u64 {
    3600
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_file() -> Option<PathBuf> {
    None
}

pub use crate::filter::FilterConfig;

impl Config {
    pub fn load(path: Option<&PathBuf>) -> Result<Self, VfsError> {
        match path {
            Some(p) => {
                let content = std::fs::read_to_string(p).map_err(|e| {
                    VfsError::Config(format!("failed to read config file {:?}: {}", p, e))
                })?;
                let config: Config = toml::from_str(&content).map_err(|e| {
                    VfsError::Config(format!("failed to parse config: {}", e))
                })?;
                Ok(config)
            }
            None => {
                // Default config
                Ok(Config {
                    backend: BackendConfig {
                        backend_dir: PathBuf::from("."),
                    },
                    mount: MountConfig {
                        drive_letter: Some('X'),
                        mount_point: None,
                    },
                    encoding: EncodingConfig {
                        source_encoding: "auto".to_string(),
                        target_encoding: "UTF-8".to_string(),
                        default_encoding: "GBK".to_string(),
                        auto_detect: true,
                        detect_sample_bytes: 8192,
                        cache_max_entries: 10000,
                        cache_ttl_seconds: 3600,
                    },
                    log: Some(LogConfig {
                        level: "info".to_string(),
                        file: None,
                    }),
                    filter: None,
                })
            }
        }
    }
}
