use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use std::path::Path;

/// Inline filter rules from TOML config.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FilterConfig {
    /// Path to filter file (default: ".evfsignore")
    #[serde(default)]
    pub filter_file: Option<String>,
    /// Inline rules, same format as `.evfsignore`
    #[serde(default)]
    pub rules: Vec<String>,
}

/// Filter rules: control which files bypass encoding conversion.
/// All files are always visible in the mount. The only distinction is:
/// - Passthrough: return raw bytes, skip encoding
/// - Visible: normal encoding conversion applies
#[derive(Debug, Clone)]
pub struct VfsFilter {
    passthrough_matchers: Vec<GlobMatcher>,
}

impl VfsFilter {
    pub fn new(filter_path: Option<&Path>, inline_rules: &[String]) -> Self {
        let mut all_lines: Vec<String> = Vec::new();

        // Read filter file if it exists
        if let Some(p) = filter_path {
            if let Ok(content) = std::fs::read_to_string(p) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    all_lines.push(trimmed.to_string());
                }
            }
        }

        // Append inline rules
        all_lines.extend(inline_rules.iter().cloned());

        let mut passthrough_matchers = Vec::new();

        for line in &all_lines {
            if line.starts_with("@passthrough ") {
                let pattern = line.strip_prefix("@passthrough ").unwrap().trim();
                if let Ok(glob) = Glob::new(pattern) {
                    passthrough_matchers.push(glob.compile_matcher());
                }
            }
        }

        Self { passthrough_matchers }
    }

    /// Whether a path should bypass encoding conversion.
    pub fn is_passthrough(&self, rel_path: &Path) -> bool {
        let path_str = rel_path.to_string_lossy().replace('\\', "/");
        for m in &self.passthrough_matchers {
            if m.is_match(&path_str) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_filter(rules: &[&str]) -> VfsFilter {
        let string_rules: Vec<String> = rules.iter().map(|s| s.to_string()).collect();
        VfsFilter::new(None, &string_rules)
    }

    #[test]
    fn test_passthrough() {
        let f = make_filter(&["@passthrough *.png"]);
        assert!(f.is_passthrough(Path::new("photo.png")));
        assert!(f.is_passthrough(Path::new("icons/icon.png")));
        assert!(!f.is_passthrough(Path::new("readme.md")));
    }

    #[test]
    fn test_multiple_passthrough() {
        let f = make_filter(&["@passthrough *.png", "@passthrough *.exe"]);
        assert!(f.is_passthrough(Path::new("image.png")));
        assert!(f.is_passthrough(Path::new("setup.exe")));
        assert!(!f.is_passthrough(Path::new("data.txt")));
    }

    #[test]
    fn test_empty_filter() {
        let f = make_filter(&[]);
        assert!(!f.is_passthrough(Path::new("anything.txt")));
    }
}
