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
#[derive(Debug, Clone)]
pub struct VfsFilter {
    patterns: Vec<Pattern>,
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

        let mut patterns = Vec::new();

        for line in &all_lines {
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

        Self { patterns }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_filter(rules: &[&str]) -> VfsFilter {
        let string_rules: Vec<String> = rules.iter().map(|s| s.to_string()).collect();
        VfsFilter::new(None, &string_rules)
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
}
