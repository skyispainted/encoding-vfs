use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use std::path::Path;

/// Whether a path should be ignored, or shown with encoding passthrough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Visible in mount, encoding-converted as usual.
    Visible,
    /// Visible in mount, but raw bytes returned without encoding conversion.
    Passthrough,
    /// Hidden from mount view entirely.
    Ignored,
}

/// Compiled filter rules loaded from .encodingvfs-ignore and/or config.
#[derive(Debug, Clone, Default)]
pub struct VfsFilter {
    ignore_matchers: Vec<GlobMatcher>,
    passthrough_matchers: Vec<GlobMatcher>,
}

/// Inline filter rules from TOML config.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FilterConfig {
    /// Path to ignore file (default: ".encodingvfs-ignore")
    #[serde(default)]
    pub ignore_file: Option<String>,
    /// Inline glob rules, same format as .encodingvfs-ignore
    #[serde(default)]
    pub rules: Vec<String>,
}

impl VfsFilter {
    pub fn new(ignore_path: Option<&Path>, inline_rules: &[String]) -> Self {
        let mut ignore_lines: Vec<String> = Vec::new();

        // Read ignore file if it exists
        if let Some(p) = ignore_path {
            if let Ok(content) = std::fs::read_to_string(p) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    ignore_lines.push(trimmed.to_string());
                }
            }
        }

        // Append inline rules
        ignore_lines.extend(inline_rules.iter().cloned());

        let mut ignore_matchers = Vec::new();
        let mut passthrough_matchers = Vec::new();

        for line in &ignore_lines {
            if line.starts_with("@passthrough ") {
                let pattern = line.strip_prefix("@passthrough ").unwrap().trim();
                if let Ok(glob) = Glob::new(pattern) {
                    if let Ok(matcher) = glob.compile_matcher() {
                        passthrough_matchers.push(matcher);
                    }
                }
            } else {
                if let Ok(glob) = Glob::new(line) {
                    if let Ok(matcher) = glob.compile_matcher() {
                        ignore_matchers.push(matcher);
                    }
                }
            }
        }

        Self {
            ignore_matchers,
            passthrough_matchers,
        }
    }

    /// Check what action to take for a given relative path.
    pub fn action(&self, rel_path: &Path) -> FilterAction {
        let path_str = rel_path.to_string_lossy().replace('\\', "/");

        // Check passthrough first
        for m in &self.passthrough_matchers {
            if m.is_match(&path_str) {
                return FilterAction::Passthrough;
            }
        }

        // Check ignore
        for m in &self.ignore_matchers {
            if m.is_match(&path_str) {
                return FilterAction::Ignored;
            }
        }

        FilterAction::Visible
    }

    /// Whether a path should be hidden from the mount view.
    pub fn is_ignored(&self, rel_path: &Path) -> bool {
        self.action(rel_path) == FilterAction::Ignored
    }

    /// Whether a path should bypass encoding conversion.
    pub fn is_passthrough(&self, rel_path: &Path) -> bool {
        self.action(rel_path) == FilterAction::Passthrough
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
    fn test_ignore_glob() {
        let f = make_filter(&["*.bin"]);
        assert!(f.is_ignored(Path::new("data.bin")));
        assert!(f.is_ignored(Path::new("sub/data.bin")));
        assert!(!f.is_ignored(Path::new("data.txt")));
    }

    #[test]
    fn test_ignore_directory() {
        let f = make_filter(&["images/"]);
        assert!(f.is_ignored(Path::new("images/logo.png")));
        assert!(f.is_ignored(Path::new("images/sub/photo.jpg")));
        assert!(!f.is_ignored(Path::new("myimages/file.txt")));
    }

    #[test]
    fn test_passthrough() {
        let f = make_filter(&["@passthrough *.png"]);
        assert!(f.is_passthrough(Path::new("photo.png")));
        assert!(f.is_passthrough(Path::new("icons/icon.png")));
        assert!(!f.is_ignored(Path::new("photo.png")));
    }

    #[test]
    fn test_priority_passthrough_over_ignore() {
        let f = make_filter(&["*.png", "@passthrough *.dat"]);
        assert!(f.is_ignored(Path::new("file.png")));
        assert!(f.is_passthrough(Path::new("file.dat")));
        assert!(!f.is_ignored(Path::new("file.dat")));
    }

    #[test]
    fn test_visible_by_default() {
        let f = make_filter(&["*.tmp"]);
        assert_eq!(f.action(Path::new("readme.md")), FilterAction::Visible);
    }
}
