use encoding_rs::{Encoding, GBK, UTF_8};
use std::io::Read;

use crate::encoding::is_likely_gbk;

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

    // Check if it's valid UTF-8 with multi-byte characters
    // If it has multi-byte UTF-8 sequences, it's likely UTF-8
    let (utf8_cow, _, utf8_errors) = UTF_8.decode(data);
    if !utf8_errors && utf8_cow.len() > 0 {
        // Check if there are actual multi-byte characters
        let has_multibyte = data.iter().any(|&b| b > 0x7F);
        if has_multibyte {
            // Has multi-byte characters and is valid UTF-8
            // But we need to check if it's more likely to be GBK
            // GBK files often have high-byte pairs that could be valid UTF-8 by chance
            // So we check if the UTF-8 decoding produces reasonable characters
            let ascii_ratio = utf8_cow.chars().filter(|c| c.is_ascii()).count() as f64 / utf8_cow.chars().count().max(1) as f64;
            // If less than 50% ASCII and valid UTF-8, likely UTF-8
            // If more than 50% ASCII, might be GBK that happens to be valid UTF-8
            if ascii_ratio < 0.5 {
                return UTF_8;
            }
        } else {
            // Pure ASCII, treat as UTF-8
            return UTF_8;
        }
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
