use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use encoding_rs::Encoding;

use crate::cache::EncodingCache;
use crate::config::EncodingConfig;
use crate::detector::detect_encoding;
use crate::encoding::{get_encoding, from_encoding, to_encoding};
use crate::error::VfsError;
use crate::filter::VfsFilter;

/// File information returned by get_file_info
#[derive(Debug)]
pub struct FileInfo {
    pub size: u64,
    pub created: SystemTime,
    pub modified: SystemTime,
    pub accessed: SystemTime,
    pub is_dir: bool,
}

/// Directory entry returned by read_dir
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: std::ffi::OsString,
    pub is_dir: bool,
    pub size: u64,
    pub modified: SystemTime,
}

/// Core encoding-aware virtual filesystem.
/// All file operations pass through this layer for transparent encoding conversion.
pub struct EncodingVfs {
    /// Backend directory containing the actual files (in original encoding)
    pub backend_dir: PathBuf,
    /// Encoding configuration
    pub encoding_config: EncodingConfig,
    /// Cache for detected encodings
    pub cache: EncodingCache,
    /// Source encoding: "auto" = detect per-file, or a specific encoding.
    pub source_encoding: &'static Encoding,
    /// Target encoding: the encoding that mounted files will appear as.
    pub target_encoding: &'static Encoding,
    /// Default encoding for auto-detection fallback.
    pub default_encoding: &'static Encoding,
    /// Path filter rules.
    pub filter: VfsFilter,
}

impl EncodingVfs {
    pub fn new(backend_dir: &Path, encoding_config: EncodingConfig) -> Result<Self, VfsError> {
        let default_encoding = get_encoding(&encoding_config.default_encoding)
            .ok_or_else(|| VfsError::Config(format!("unsupported encoding: {}", encoding_config.default_encoding)))?;

        let target_encoding = get_encoding(&encoding_config.target_encoding)
            .ok_or_else(|| VfsError::Config(format!("unsupported target encoding: {}", encoding_config.target_encoding)))?;

        let source_encoding = if encoding_config.source_encoding.eq_ignore_ascii_case("auto") {
            // In auto mode, source is resolved per-file via detect_encoding
            // We use default_encoding as a placeholder; actual resolution happens in read_file
            default_encoding
        } else {
            get_encoding(&encoding_config.source_encoding)
                .ok_or_else(|| VfsError::Config(format!("unsupported source encoding: {}", encoding_config.source_encoding)))?
        };

        let cache = EncodingCache::new(
            encoding_config.cache_max_entries,
            encoding_config.cache_ttl_seconds,
        );

        let filter = match &encoding_config.filter {
            Some(fc) => {
                // Add default hidden rules (.git directory) if not explicitly overridden
                let mut hidden_rules = vec![".git/".to_string()];
                hidden_rules.extend(fc.hidden.clone());
                VfsFilter::new(&fc.rules, &hidden_rules)
            }
            None => {
                // Default: hide .git directory
                let default_hidden = vec![".git/".to_string()];
                VfsFilter::new(&[], &default_hidden)
            }
        };

        Ok(Self {
            backend_dir: backend_dir.to_path_buf(),
            encoding_config,
            cache,
            source_encoding,
            target_encoding,
            default_encoding,
            filter,
        })
    }

    /// Resolve the full path for a relative path within the backend directory
    pub fn full_path(&self, rel_path: &Path) -> PathBuf {
        self.backend_dir.join(rel_path)
    }

    /// Detect encoding for a file (uses cache if available)
    pub fn resolve_encoding(&self, full_path: &Path) -> &'static Encoding {
        // Check cache first
        if let Some(enc) = self.cache.get(full_path) {
            return enc;
        }

        // Read sample bytes and detect
        let sample = self.read_backend_bytes(full_path, 0, self.encoding_config.detect_sample_bytes)
            .unwrap_or_default();

        let enc = detect_encoding(&sample, self.default_encoding);
        self.cache.insert(full_path, enc);
        enc
    }

    /// Read raw bytes from backend file (no encoding conversion)
    pub fn read_backend_bytes(&self, full_path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, VfsError> {
        if !full_path.exists() {
            return Err(VfsError::NotFound(full_path.to_path_buf()));
        }

        let mut file = fs::File::open(full_path)?;
        file.seek(SeekFrom::Start(offset))?;

        let mut buffer = vec![0u8; len];
        let n = file.read(&mut buffer)?;
        buffer.truncate(n);

        Ok(buffer)
    }

    /// Write raw bytes to backend file (no encoding conversion)
    pub fn write_backend_bytes(&self, full_path: &Path, offset: u64, data: &[u8]) -> Result<(), VfsError> {
        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if offset == 0 {
            // Full write: truncate and write from start
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(full_path)?;
            file.write_all(data)?;
            file.flush()?;
        } else {
            // Partial write: seek and write, then truncate remainder
            let mut file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(full_path)?;
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(data)?;
            // Truncate file to new end position
            file.set_len(offset + data.len() as u64)?;
        }

        // Invalidate cache since file was modified
        self.cache.invalidate(full_path);

        Ok(())
    }

    /// Read file content and convert from source encoding to target encoding.
    /// In "auto" source mode, detects per-file encoding.
    pub fn read_file(&self, rel_path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, VfsError> {
        // Check if path is hidden
        if self.filter.is_hidden(rel_path) {
            return Err(VfsError::NotFound(rel_path.to_path_buf()));
        }

        let full_path = self.full_path(rel_path);

        // Read raw bytes from backend
        let raw = self.read_backend_bytes(&full_path, offset, len)?;

        // Passthrough: return raw bytes without encoding conversion
        if self.filter.is_passthrough(rel_path) {
            return Ok(raw);
        }

        // Determine source encoding
        let src_enc = if self.encoding_config.source_encoding.eq_ignore_ascii_case("auto") {
            self.resolve_encoding(&full_path)
        } else {
            self.source_encoding
        };

        // Convert to target encoding
        let (converted, had_errors) = to_encoding(&raw, src_enc, self.target_encoding);

        if had_errors {
            tracing::warn!(
                path = ?rel_path,
                source = src_enc.name(),
                target = self.target_encoding.name(),
                "encoding had errors during conversion, using replacement characters"
            );
        }

        Ok(converted)
    }

    /// Write file content, converting from target encoding to source encoding.
    /// In "auto" source mode, preserves the file's existing encoding.
    pub fn write_file(&self, rel_path: &Path, offset: u64, data: &[u8]) -> Result<u64, VfsError> {
        // Check if path is hidden
        if self.filter.is_hidden(rel_path) {
            return Err(VfsError::NotFound(rel_path.to_path_buf()));
        }

        let full_path = self.full_path(rel_path);

        // Determine source encoding for the backend file
        let src_enc = if self.encoding_config.source_encoding.eq_ignore_ascii_case("auto") {
            // In auto mode, write to the file's existing encoding (preserve it)
            if full_path.exists() {
                self.resolve_encoding(&full_path)
            } else {
                self.default_encoding
            }
        } else {
            self.source_encoding
        };

        // Convert from target encoding to source encoding
        let (decoded, had_errors) = from_encoding(data, self.target_encoding, src_enc);

        if had_errors {
            tracing::warn!(
                path = ?rel_path,
                source = self.target_encoding.name(),
                target = src_enc.name(),
                "encoding had errors during conversion, some characters may be lost"
            );
        }

        // For offset > 0, we can't write encoded bytes at the same offset because
        // byte positions differ between encodings. Read entire file as UTF-8,
        // splice in the new data at the UTF-8 offset, then re-encode everything.
        if offset > 0 {
            let file_info = self.get_file_info(rel_path)?;
            let file_size = file_info.size as usize;

            // Read existing content as target encoding (UTF-8)
            let existing = self.read_file(rel_path, 0, file_size)?;

            // Splice: replace at target-encoding offset
            let utf8_offset = offset as usize;
            let utf8_len = data.len();
            let mut combined = Vec::with_capacity(utf8_offset + utf8_len);
            if utf8_offset < existing.len() {
                combined.extend_from_slice(&existing[..utf8_offset]);
            } else {
                combined.extend_from_slice(&existing);
                // Pad with spaces if offset is past end
                while combined.len() < utf8_offset {
                    combined.push(b' ');
                }
            }
            combined.extend_from_slice(data);
            // Truncate after the inserted data
            let new_end = utf8_offset + utf8_len;
            if new_end < existing.len() {
                combined.extend_from_slice(&existing[new_end..]);
            }

            // Re-encode entire result to source encoding
            let (full_encoded, _) = from_encoding(&combined, self.target_encoding, src_enc);
            self.write_backend_bytes(&full_path, 0, &full_encoded)?;
            return Ok(full_encoded.len() as u64);
        }

        // Write to backend (offset 0: truncate and write)
        self.write_backend_bytes(&full_path, 0, &decoded)?;

        Ok(decoded.len() as u64)
    }

    /// Get file information.
    /// Returns converted file size (target encoding) for text files.
    pub fn get_file_info(&self, rel_path: &Path) -> Result<FileInfo, VfsError> {
        // Check if path is hidden
        if self.filter.is_hidden(rel_path) {
            return Err(VfsError::NotFound(rel_path.to_path_buf()));
        }

        let full_path = self.full_path(rel_path);
        let metadata = fs::metadata(&full_path)?;

        // For text files, calculate converted size
        let size = if self.filter.is_passthrough(rel_path) {
            metadata.len()
        } else {
            // Read entire file and calculate converted size
            let raw = fs::read(&full_path)?;
            let src_enc = if self.encoding_config.source_encoding.eq_ignore_ascii_case("auto") {
                self.resolve_encoding(&full_path)
            } else {
                self.source_encoding
            };
            let (converted, _) = to_encoding(&raw, src_enc, self.target_encoding);
            converted.len() as u64
        };

        Ok(FileInfo {
            size,
            created: metadata.created()?,
            modified: metadata.modified()?,
            accessed: metadata.accessed()?,
            is_dir: false,
        })
    }

    /// Read directory entries.
    pub fn read_dir(&self, rel_path: &Path) -> Result<Vec<DirEntry>, VfsError> {
        let full_path = self.full_path(rel_path);
        let entries = fs::read_dir(&full_path)?;

        entries
            .filter_map(|e| {
                let e = match e {
                    Ok(e) => e,
                    Err(_) => return Some(Err(VfsError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "failed to read directory entry",
                    )))),
                };
                let metadata = match e.metadata() {
                    Ok(m) => m,
                    Err(_) => return Some(Err(VfsError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "failed to read metadata",
                    )))),
                };

                // Check if this entry should be hidden
                let entry_rel_path = rel_path.join(e.file_name());
                if self.filter.is_hidden(&entry_rel_path) {
                    return None; // Skip hidden entries
                }

                Some(Ok(DirEntry {
                    name: e.file_name(),
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                    modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                }))
            })
            .collect()
    }

    /// Create a directory
    pub fn create_dir(&self, rel_path: &Path) -> Result<(), VfsError> {
        let full_path = self.full_path(rel_path);
        fs::create_dir_all(&full_path)?;
        Ok(())
    }

    /// Remove a file or empty directory
    pub fn remove(&self, rel_path: &Path) -> Result<(), VfsError> {
        let full_path = self.full_path(rel_path);
        if full_path.is_dir() {
            fs::remove_dir(&full_path)?;
        } else {
            fs::remove_file(&full_path)?;
        }
        self.cache.invalidate(&full_path);
        Ok(())
    }

    /// Rename/move a file or directory
    pub fn rename(&self, from: &Path, to: &Path) -> Result<(), VfsError> {
        let from_full = self.full_path(from);
        let to_full = self.full_path(to);
        fs::rename(&from_full, &to_full)?;
        self.cache.invalidate(&from_full);
        Ok(())
    }

    /// Check if a file exists
    pub fn exists(&self, rel_path: &Path) -> bool {
        // Hidden files are considered non-existent
        if self.filter.is_hidden(rel_path) {
            return false;
        }
        self.full_path(rel_path).exists()
    }

    /// Check if a path is a directory
    pub fn is_dir(&self, rel_path: &Path) -> bool {
        // Hidden paths are considered non-existent
        if self.filter.is_hidden(rel_path) {
            return false;
        }
        self.full_path(rel_path).is_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_gbk_file() {
        // Create a temp directory with a GBK file
        let temp_dir = std::env::temp_dir().join("encoding_vfs_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let gbk_file = temp_dir.join("test.txt");
        let gbk_bytes = b"\xC4\xE3\xBA\xC3\xCA\xC0\xBD\xE7"; // "你好世界" in GBK

        let mut f = fs::File::create(&gbk_file).unwrap();
        f.write_all(gbk_bytes).unwrap();

        let config = EncodingConfig {
            source_encoding: "auto".to_string(),
            target_encoding: "UTF-8".to_string(),
            default_encoding: "GBK".to_string(),
            auto_detect: true,
            detect_sample_bytes: 8192,
            cache_max_entries: 100,
            cache_ttl_seconds: 60,
            filter: None,
        };

        let vfs = EncodingVfs::new(&temp_dir, config).unwrap();
        let utf8_data = vfs.read_file(Path::new("test.txt"), 0, 100).unwrap();
        let utf8_str = String::from_utf8(utf8_data).unwrap();

        assert_eq!(utf8_str, "你好世界");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_write_utf8_to_gbk() {
        // Create a temp directory
        let temp_dir = std::env::temp_dir().join("encoding_vfs_write_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let gbk_file = temp_dir.join("write_test.txt");

        // First write a small GBK file to establish encoding
        let initial_gbk = b"\xC4\xE3"; // "你" in GBK
        let mut f = fs::File::create(&gbk_file).unwrap();
        f.write_all(initial_gbk).unwrap();

        let config = EncodingConfig {
            source_encoding: "auto".to_string(),
            target_encoding: "UTF-8".to_string(),
            default_encoding: "GBK".to_string(),
            auto_detect: true,
            detect_sample_bytes: 8192,
            cache_max_entries: 100,
            cache_ttl_seconds: 60,
            filter: None,
        };

        let vfs = EncodingVfs::new(&temp_dir, config).unwrap();

        // Write UTF-8 content
        let utf8_content = "你好，世界！";
        let written = vfs.write_file(
            Path::new("write_test.txt"),
            0,
            utf8_content.as_bytes(),
        ).unwrap();

        assert!(written > 0);

        // Verify the file was written correctly in GBK
        let raw = fs::read(&gbk_file).unwrap();
        let (decoded, _, _) = encoding_rs::GBK.decode(&raw);
        assert_eq!(decoded, utf8_content);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
