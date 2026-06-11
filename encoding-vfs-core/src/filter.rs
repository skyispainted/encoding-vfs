use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use std::path::Path;

/// Inline filter rules from TOML config.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FilterConfig {
    /// Inline rules for passthrough (skip encoding conversion), .gitignore-style syntax
    #[serde(default)]
    pub rules: Vec<String>,
    /// Hidden rules: files/directories matching these patterns will be completely hidden
    /// from the mounted filesystem. Uses .gitignore-style syntax.
    #[serde(default)]
    pub hidden: Vec<String>,
}

/// A single filter pattern, matching .gitignore-style semantics.
#[derive(Debug, Clone)]
struct Pattern {
    /// Globset matcher. If `None`, this is a "match-all" pattern (`**`).
    matcher: Option<GlobMatcher>,
    /// `true` means "do NOT passthrough" (negation, like `!` in .gitignore).
    negated: bool,
}

/// Filter rules: control which files bypass encoding conversion.
/// All files are always visible. Patterns matching a file cause it to
/// skip encoding conversion (return raw bytes). Format follows .gitignore style.
///
/// Rules are evaluated in order; the last matching pattern wins.
/// Default (no match) = normal encoding conversion.
/// `!pattern` negates a previous match (restore encoding).
///
/// Hidden rules: files/directories matching these patterns are completely
/// hidden from the mounted filesystem.
#[derive(Debug, Clone)]
pub struct VfsFilter {
    patterns: Vec<Pattern>,
    hidden_patterns: Vec<Pattern>,
}

impl VfsFilter {
    pub fn new(inline_rules: &[String], hidden_rules: &[String]) -> Self {
        let patterns = Self::compile_patterns(inline_rules);
        let hidden_patterns = Self::compile_patterns(hidden_rules);

        Self { patterns, hidden_patterns }
    }

    fn compile_patterns(lines: &[String]) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        for line in lines {
            let (negated, pattern) = if let Some(rest) = line.strip_prefix('!') {
                (true, rest.trim())
            } else {
                (false, line.as_str())
            };

            if pattern == "**" {
                // `**` matches everything — store as None matcher (special case).
                patterns.push(Pattern {
                    matcher: None,
                    negated,
                });
            } else {
                // `.gitignore` 语义：`dir/` 表示忽略整个目录下的所有文件。
                // globset 不直接支持这种语义，需要展开为 `{dir,dir/**}`。
                let expanded: Vec<String> = if pattern.ends_with('/') {
                    let dir = pattern.trim_end_matches('/');
                    vec![dir.to_string(), format!("{dir}/**")]
                } else {
                    vec![pattern.to_string()]
                };

                for p in expanded {
                    if let Ok(glob) = Glob::new(&p) {
                        patterns.push(Pattern {
                            matcher: Some(glob.compile_matcher()),
                            negated,
                        });
                    }
                }
            }
        }

        patterns
    }

    /// Whether a path should bypass encoding conversion.
    /// Rules are evaluated in order; last matching pattern wins.
    pub fn is_passthrough(&self, rel_path: &Path) -> bool {
        let path_str = rel_path.to_string_lossy().replace('\\', "/");
        let mut result = false; // default: normal encoding

        for p in &self.patterns {
            let matched = p.matcher.as_ref().is_none() || p.matcher.as_ref().unwrap().is_match(&path_str);
            if matched {
                if p.negated {
                    result = false; // negation: restore encoding
                } else {
                    result = true; // skip encoding
                }
            }
        }

        result
    }

    /// Whether a path should be completely hidden from the mounted filesystem.
    /// Hidden rules use the same .gitignore-style syntax.
    pub fn is_hidden(&self, rel_path: &Path) -> bool {
        let path_str = rel_path.to_string_lossy().replace('\\', "/");

        for p in &self.hidden_patterns {
            let matched = p.matcher.as_ref().is_none() || p.matcher.as_ref().unwrap().is_match(&path_str);
            if matched && !p.negated {
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
        VfsFilter::new(&string_rules, &[])
    }

    fn make_filter_with_hidden(rules: &[&str], hidden: &[&str]) -> VfsFilter {
        let string_rules: Vec<String> = rules.iter().map(|s| s.to_string()).collect();
        let hidden_rules: Vec<String> = hidden.iter().map(|s| s.to_string()).collect();
        VfsFilter::new(&string_rules, &hidden_rules)
    }

    #[test]
    fn test_basic_passthrough() {
        let f = make_filter(&["*.png"]);
        assert!(f.is_passthrough(Path::new("photo.png")));
        assert!(f.is_passthrough(Path::new("icons/icon.png")));
        assert!(!f.is_passthrough(Path::new("readme.md")));
    }

    #[test]
    fn test_negation() {
        let f = make_filter(&["*.png", "!logo.png"]);
        assert!(f.is_passthrough(Path::new("photo.png")));
        assert!(f.is_passthrough(Path::new("icons/icon.png")));
        // Negated: logo.png uses encoding even though *.png matches
        assert!(!f.is_passthrough(Path::new("logo.png")));
    }

    #[test]
    fn test_negation_before_positive() {
        let f = make_filter(&["*.png", "!logo.png", "logo.png"]);
        // logo.png matches three times, last one is positive → passthrough
        assert!(f.is_passthrough(Path::new("logo.png")));
    }

    #[test]
    fn test_multiple() {
        let f = make_filter(&["*.png", "*.exe"]);
        assert!(f.is_passthrough(Path::new("image.png")));
        assert!(f.is_passthrough(Path::new("setup.exe")));
        assert!(!f.is_passthrough(Path::new("data.txt")));
    }

    #[test]
    fn test_directory() {
        let f = make_filter(&["assets/"]);
        assert!(f.is_passthrough(Path::new("assets/logo.png")));
        assert!(f.is_passthrough(Path::new("assets/sub/photo.jpg")));
        assert!(!f.is_passthrough(Path::new("myassets/file.txt")));
    }

    #[test]
    fn test_empty_filter() {
        let f = make_filter(&[]);
        assert!(!f.is_passthrough(Path::new("anything.txt")));
    }

    #[test]
    fn test_double_star_matches_all() {
        // `**` matches every file including root-level and deep subdirectory files
        let f = make_filter(&["**"]);
        assert!(f.is_passthrough(Path::new("main.cpp")));
        assert!(f.is_passthrough(Path::new("src/main.cpp")));
        assert!(f.is_passthrough(Path::new("a/b/c/d.txt")));
    }

    #[test]
    fn test_double_star_with_negation() {
        // Only convert .h and .cpp, everything else passthrough
        let f = make_filter(&["**", "!*.h", "!*.cpp"]);
        assert!(!f.is_passthrough(Path::new("main.cpp")));
        assert!(!f.is_passthrough(Path::new("src/main.cpp")));
        assert!(!f.is_passthrough(Path::new("src/header.h")));
        assert!(f.is_passthrough(Path::new("data.xml")));
        assert!(f.is_passthrough(Path::new("src/data.xml")));
        assert!(f.is_passthrough(Path::new(".git/HEAD")));
    }

    #[test]
    fn test_hidden_basic() {
        let f = make_filter_with_hidden(&[], &[".git/"]);
        assert!(f.is_hidden(Path::new(".git")));
        assert!(f.is_hidden(Path::new(".git/HEAD")));
        assert!(f.is_hidden(Path::new(".git/config")));
        assert!(!f.is_hidden(Path::new("src")));
        assert!(!f.is_hidden(Path::new("README.md")));
    }

    #[test]
    fn test_hidden_multiple() {
        let f = make_filter_with_hidden(&[], &[".git/", "*.tmp"]);
        assert!(f.is_hidden(Path::new(".git")));
        assert!(f.is_hidden(Path::new(".git/HEAD")));
        assert!(f.is_hidden(Path::new("cache.tmp")));
        assert!(f.is_hidden(Path::new("build/output.tmp")));
        assert!(!f.is_hidden(Path::new("src")));
        assert!(!f.is_hidden(Path::new("main.cpp")));
    }
}
