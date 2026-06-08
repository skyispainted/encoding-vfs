use encoding_rs::Encoding;

pub use encoding_rs::{GBK, UTF_8};

/// Convert bytes from source encoding to UTF-8.
/// Returns (utf8_bytes, had_errors).
pub fn to_utf8(data: &[u8], src_encoding: &'static Encoding) -> (Vec<u8>, bool) {
    let (cow, _, had_errors) = src_encoding.decode(data);
    (cow.into_owned().into_bytes(), had_errors)
}

/// Convert UTF-8 bytes to target encoding.
/// Returns (encoded_bytes, had_errors).
pub fn from_utf8(data: &[u8], target_encoding: &'static Encoding) -> (Vec<u8>, bool) {
    let s = std::str::from_utf8(data).unwrap_or("");
    let (cow, _, had_errors) = target_encoding.encode(s);
    (cow.into_owned(), had_errors)
}

/// Convert bytes from src_encoding to target_encoding via Unicode intermediate.
/// Returns (converted_bytes, had_errors).
pub fn to_encoding(data: &[u8], src_encoding: &'static Encoding, target_encoding: &'static Encoding) -> (Vec<u8>, bool) {
    let (text, _, decode_errors) = src_encoding.decode(data);
    let (cow, _, encode_errors) = target_encoding.encode(&text);
    (cow.into_owned(), decode_errors || encode_errors)
}

/// Convert bytes from source_encoding to target_encoding via Unicode intermediate.
/// Returns (converted_bytes, had_errors).
pub fn from_encoding(data: &[u8], source_encoding: &'static Encoding, target_encoding: &'static Encoding) -> (Vec<u8>, bool) {
    let (text, _, decode_errors) = source_encoding.decode(data);
    let (cow, _, encode_errors) = target_encoding.encode(&text);
    (cow.into_owned(), decode_errors || encode_errors)
}

/// Get encoding by name. Returns None if not supported.
pub fn get_encoding(name: &str) -> Option<&'static Encoding> {
    let name_upper = name.to_uppercase();
    match name_upper.as_str() {
        "UTF-8" | "UTF8" => Some(UTF_8),
        "GBK" | "CP936" | "GB2312" | "GB18030" => Some(GBK),
        "BIG5" => Some(encoding_rs::BIG5),
        "EUC-JP" => Some(encoding_rs::EUC_JP),
        "EUC-KR" => Some(encoding_rs::EUC_KR),
        "ISO-2022-JP" => Some(encoding_rs::ISO_2022_JP),
        "KOI8-R" => Some(encoding_rs::KOI8_R),
        "WINDOWS-1252" | "CP1252" => Some(encoding_rs::WINDOWS_1252),
        "UTF-16LE" => Some(encoding_rs::UTF_16LE),
        "UTF-16BE" => Some(encoding_rs::UTF_16BE),
        "IBM866" => Some(encoding_rs::IBM866),
        "ISO-8859-10" => Some(encoding_rs::ISO_8859_10),
        "ISO-8859-13" => Some(encoding_rs::ISO_8859_13),
        "ISO-8859-14" => Some(encoding_rs::ISO_8859_14),
        "ISO-8859-15" => Some(encoding_rs::ISO_8859_15),
        "ISO-8859-16" => Some(encoding_rs::ISO_8859_16),
        "ISO-8859-2" => Some(encoding_rs::ISO_8859_2),
        "ISO-8859-3" => Some(encoding_rs::ISO_8859_3),
        "ISO-8859-4" => Some(encoding_rs::ISO_8859_4),
        "ISO-8859-5" => Some(encoding_rs::ISO_8859_5),
        "ISO-8859-6" => Some(encoding_rs::ISO_8859_6),
        "ISO-8859-7" => Some(encoding_rs::ISO_8859_7),
        "ISO-8859-8" => Some(encoding_rs::ISO_8859_8),
        "MACINTOSH" => Some(encoding_rs::MACINTOSH),
        "SHIFT_JIS" => Some(encoding_rs::SHIFT_JIS),
        "X-MAC-CYRILLIC" => Some(encoding_rs::X_MAC_CYRILLIC),
        _ => Encoding::for_label(name_upper.as_bytes()),
    }
}

/// Check if data is likely UTF-8 already
pub fn is_likely_utf8(data: &[u8]) -> bool {
    let (cow, _, had_errors) = UTF_8.decode(data);
    !had_errors && cow.len() > 0
}

/// Check if data is likely GBK
pub fn is_likely_gbk(data: &[u8]) -> bool {
    let (_, _, had_errors) = GBK.decode(data);
    !had_errors
}
