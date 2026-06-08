use encoding_rs::{Encoding, GBK, UTF_8};
use std::io::Read;

use crate::encoding::{is_likely_gbk, is_likely_utf8};

/// Detect encoding from file content bytes.
/// Returns the detected encoding or the default encoding if detection fails.
pub fn detect_encoding(data: &[u8], default: &'static Encoding) -> &'static Encoding {
    if data.is_empty() {
        return default;
    }

    // Check BOM first
    if let Some(enc) = detect_bom(data) {
        return enc;
    }

    // If looks like valid UTF-8, prefer UTF-8
    if is_likely_utf8(data) {
        return UTF_8;
    }

    // If looks like valid GBK, prefer GBK
    if is_likely_gbk(data) {
        return GBK;
    }

    // Fallback to default
    default
}

/// Detect encoding from BOM (Byte Order Mark)
pub fn detect_bom(data: &[u8]) -> Option<&'static Encoding> {
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Some(UTF_8)
    } else if data.starts_with(&[0xFF, 0xFE]) {
        Some(encoding_rs::UTF_16LE)
    } else if data.starts_with(&[0xFE, 0xFF]) {
        Some(encoding_rs::UTF_16BE)
    } else {
        None
    }
}

/// Detect encoding from a file by reading the first N bytes
pub fn detect_encoding_from_file(path: &std::path::Path, sample_bytes: usize, default: &'static Encoding) -> &'static Encoding {
    match std::fs::File::open(path) {
        Ok(mut file) => {
            let mut buffer = vec![0u8; sample_bytes];
            match file.read(&mut buffer) {
                Ok(n) => {
                    buffer.truncate(n);
                    detect_encoding(&buffer, default)
                }
                Err(_) => default,
            }
        }
        Err(_) => default,
    }
}

/// Detect encoding from a reader (useful for stream detection)
pub fn detect_encoding_from_reader<R: Read>(mut reader: R, sample_bytes: usize, default: &'static Encoding) -> &'static Encoding {
    let mut buffer = vec![0u8; sample_bytes];
    match reader.read(&mut buffer) {
        Ok(n) => {
            buffer.truncate(n);
            detect_encoding(&buffer, default)
        }
        Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf8_bom() {
        let data = &[0xEF, 0xBB, 0xBF, 0x48, 0x65, 0x6c, 0x6c, 0x6f];
        assert_eq!(detect_encoding(data, GBK), UTF_8);
    }

    #[test]
    fn test_detect_utf16_le_bom() {
        let data = &[0xFF, 0xFE, 0x48, 0x00, 0x65, 0x00];
        assert_eq!(detect_encoding(data, GBK), encoding_rs::UTF_16LE);
    }

    #[test]
    fn test_detect_plain_utf8() {
        let data = b"Hello, World!";
        assert_eq!(detect_encoding(data, GBK), UTF_8);
    }

    #[test]
    fn test_detect_fallback() {
        let data: &[u8] = &[];
        assert_eq!(detect_encoding(data, GBK), GBK);
    }
}
