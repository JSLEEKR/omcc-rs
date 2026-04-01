//! Magic keyword detection: strip code blocks, match keywords, trigger mode changes.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// A detected keyword match with its mode activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeywordMatch {
    pub keyword: String,
    pub mode: KeywordMode,
    pub position: usize,
}

/// Modes that can be activated by keywords.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeywordMode {
    Autopilot,
    Ralph,
    Ultrawork,
    Ultrathink,
    Compact,
    Research,
    Plan,
    Debug,
    Custom(String),
}

impl std::fmt::Display for KeywordMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeywordMode::Autopilot => write!(f, "autopilot"),
            KeywordMode::Ralph => write!(f, "ralph"),
            KeywordMode::Ultrawork => write!(f, "ultrawork"),
            KeywordMode::Ultrathink => write!(f, "ultrathink"),
            KeywordMode::Compact => write!(f, "compact"),
            KeywordMode::Research => write!(f, "research"),
            KeywordMode::Plan => write!(f, "plan"),
            KeywordMode::Debug => write!(f, "debug"),
            KeywordMode::Custom(name) => write!(f, "custom:{name}"),
        }
    }
}

/// Built-in keyword definitions.
struct KeywordDef {
    patterns: Vec<&'static str>,
    mode: KeywordMode,
}

/// Get the built-in keyword definitions.
fn builtin_keywords() -> Vec<KeywordDef> {
    vec![
        KeywordDef {
            patterns: vec!["autopilot", "auto pilot", "auto-pilot"],
            mode: KeywordMode::Autopilot,
        },
        KeywordDef {
            patterns: vec!["ralph", "persistent", "persist mode"],
            mode: KeywordMode::Ralph,
        },
        KeywordDef {
            patterns: vec!["ultrawork", "ulw", "ultra work"],
            mode: KeywordMode::Ultrawork,
        },
        KeywordDef {
            patterns: vec!["ultrathink", "ultra think", "deep think"],
            mode: KeywordMode::Ultrathink,
        },
        KeywordDef {
            patterns: vec!["compact", "compress context"],
            mode: KeywordMode::Compact,
        },
        KeywordDef {
            patterns: vec!["research mode", "deep research"],
            mode: KeywordMode::Research,
        },
        KeywordDef {
            patterns: vec!["plan mode", "planning mode"],
            mode: KeywordMode::Plan,
        },
        KeywordDef {
            patterns: vec!["debug mode", "debugging mode"],
            mode: KeywordMode::Debug,
        },
    ]
}

/// The keyword detector engine.
pub struct KeywordDetector {
    /// Compiled regex for stripping code blocks.
    code_block_re: Regex,
    /// Regex for detecting non-Latin scripts (CJK, etc.).
    non_latin_re: Regex,
    /// Custom keyword definitions (in addition to builtins).
    custom_keywords: Vec<(String, KeywordMode)>,
}

impl KeywordDetector {
    /// Create a new keyword detector.
    pub fn new() -> Self {
        Self {
            code_block_re: Regex::new(r"(?s)```.*?```").unwrap(),
            non_latin_re: Regex::new(r"[\p{Han}\p{Hangul}\p{Hiragana}\p{Katakana}]").unwrap(),
            custom_keywords: Vec::new(),
        }
    }

    /// Register a custom keyword trigger.
    pub fn add_custom_keyword(&mut self, trigger: &str, mode: KeywordMode) {
        self.custom_keywords.push((trigger.to_lowercase(), mode));
    }

    /// Strip code blocks from text to avoid false matches.
    pub fn strip_code_blocks(&self, text: &str) -> String {
        self.code_block_re.replace_all(text, "").to_string()
    }

    /// Check if text contains non-Latin characters.
    pub fn has_non_latin(&self, text: &str) -> bool {
        self.non_latin_re.is_match(text)
    }

    /// Detect all matching keywords in the input text.
    /// Returns matches sorted by position.
    pub fn detect(&self, text: &str) -> Vec<KeywordMatch> {
        let cleaned = self.strip_code_blocks(text);
        let lower = cleaned.to_lowercase();
        let mut matches = Vec::new();

        // Check built-in keywords
        for def in builtin_keywords() {
            for pattern in &def.patterns {
                if let Some(pos) = lower.find(pattern) {
                    matches.push(KeywordMatch {
                        keyword: pattern.to_string(),
                        mode: def.mode.clone(),
                        position: pos,
                    });
                    break; // Only first pattern match per keyword
                }
            }
        }

        // Check custom keywords
        for (trigger, mode) in &self.custom_keywords {
            if let Some(pos) = lower.find(trigger.as_str()) {
                matches.push(KeywordMatch {
                    keyword: trigger.clone(),
                    mode: mode.clone(),
                    position: pos,
                });
            }
        }

        // Sort by position
        matches.sort_by_key(|m| m.position);
        matches
    }

    /// Detect keywords and return only the primary (first) mode.
    pub fn detect_primary(&self, text: &str) -> Option<KeywordMatch> {
        self.detect(text).into_iter().next()
    }

    /// Check if a specific mode is triggered.
    pub fn is_mode_triggered(&self, text: &str, mode: &KeywordMode) -> bool {
        self.detect(text).iter().any(|m| &m.mode == mode)
    }

    /// Check if the task is too small for keyword modes.
    /// Short prompts (<20 chars) that are just the keyword shouldn't trigger complex modes.
    pub fn is_task_too_small(&self, text: &str) -> bool {
        let cleaned = self.strip_code_blocks(text).trim().to_string();
        cleaned.len() < 20
    }
}

impl Default for KeywordDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_autopilot() {
        let detector = KeywordDetector::new();
        let matches = detector.detect("Please run in autopilot mode");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].mode, KeywordMode::Autopilot);
    }

    #[test]
    fn test_detect_ralph() {
        let detector = KeywordDetector::new();
        let matches = detector.detect("Use ralph to fix this");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].mode, KeywordMode::Ralph);
    }

    #[test]
    fn test_detect_ultrawork() {
        let detector = KeywordDetector::new();
        let matches = detector.detect("ulw fix all bugs");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].mode, KeywordMode::Ultrawork);
    }

    #[test]
    fn test_detect_multiple_keywords() {
        let detector = KeywordDetector::new();
        let matches = detector.detect("autopilot ralph ultrawork");
        assert!(matches.len() >= 3);
    }

    #[test]
    fn test_case_insensitive() {
        let detector = KeywordDetector::new();
        let matches = detector.detect("AUTOPILOT mode please");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].mode, KeywordMode::Autopilot);
    }

    #[test]
    fn test_strip_code_blocks() {
        let detector = KeywordDetector::new();
        let text = "Please fix ```rust\nlet autopilot = true;\n``` the code";
        let matches = detector.detect(text);
        // "autopilot" is inside a code block, should not match
        assert!(matches.is_empty());
    }

    #[test]
    fn test_no_matches() {
        let detector = KeywordDetector::new();
        let matches = detector.detect("Just a normal request to write some code");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_detect_primary() {
        let detector = KeywordDetector::new();
        let primary = detector.detect_primary("autopilot ralph");
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().mode, KeywordMode::Autopilot);
    }

    #[test]
    fn test_detect_primary_none() {
        let detector = KeywordDetector::new();
        let primary = detector.detect_primary("normal request");
        assert!(primary.is_none());
    }

    #[test]
    fn test_is_mode_triggered() {
        let detector = KeywordDetector::new();
        assert!(detector.is_mode_triggered("run in autopilot", &KeywordMode::Autopilot));
        assert!(!detector.is_mode_triggered("normal request", &KeywordMode::Autopilot));
    }

    #[test]
    fn test_custom_keyword() {
        let mut detector = KeywordDetector::new();
        detector.add_custom_keyword("megamode", KeywordMode::Custom("mega".to_string()));
        let matches = detector.detect("activate megamode now");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].mode, KeywordMode::Custom("mega".to_string()));
    }

    #[test]
    fn test_non_latin_detection() {
        let detector = KeywordDetector::new();
        assert!(detector.has_non_latin("한국어 텍스트"));
        assert!(detector.has_non_latin("日本語"));
        assert!(detector.has_non_latin("中文"));
        assert!(!detector.has_non_latin("English only"));
    }

    #[test]
    fn test_task_too_small() {
        let detector = KeywordDetector::new();
        assert!(detector.is_task_too_small("autopilot"));
        assert!(!detector.is_task_too_small("autopilot: refactor the entire codebase and write tests"));
    }

    #[test]
    fn test_keyword_mode_display() {
        assert_eq!(KeywordMode::Autopilot.to_string(), "autopilot");
        assert_eq!(KeywordMode::Ralph.to_string(), "ralph");
        assert_eq!(KeywordMode::Custom("x".to_string()).to_string(), "custom:x");
    }

    #[test]
    fn test_sorted_by_position() {
        let detector = KeywordDetector::new();
        let matches = detector.detect("ralph then autopilot mode");
        assert!(matches.len() >= 2);
        assert!(matches[0].position < matches[1].position);
    }

    #[test]
    fn test_auto_pilot_hyphenated() {
        let detector = KeywordDetector::new();
        let matches = detector.detect("Use auto-pilot for this task");
        assert_eq!(matches[0].mode, KeywordMode::Autopilot);
    }

    #[test]
    fn test_deep_think_alias() {
        let detector = KeywordDetector::new();
        let matches = detector.detect("Enable deep think for analysis");
        assert_eq!(matches[0].mode, KeywordMode::Ultrathink);
    }

    #[test]
    fn test_compact_keyword() {
        let detector = KeywordDetector::new();
        assert!(detector.is_mode_triggered("compact the context", &KeywordMode::Compact));
    }

    #[test]
    fn test_research_mode() {
        let detector = KeywordDetector::new();
        assert!(detector.is_mode_triggered("research mode please", &KeywordMode::Research));
    }

    #[test]
    fn test_empty_input() {
        let detector = KeywordDetector::new();
        assert!(detector.detect("").is_empty());
    }
}
