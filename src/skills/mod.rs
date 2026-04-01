//! Skill learning: detect patterns from successful tool uses,
//! SHA-256 dedup, confidence scoring, pattern storage.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// A learned skill pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub pattern: String,
    pub triggers: Vec<String>,
    pub tags: Vec<String>,
    pub confidence: f64,
    pub usage_count: u32,
    pub hash: String,
    pub created_at: String,
}

/// A tool use event for pattern detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseEvent {
    pub tool_name: String,
    pub input: String,
    pub output: String,
    pub success: bool,
    pub duration_ms: u64,
}

/// Confidence adjustment parameters.
const BASE_CONFIDENCE: f64 = 50.0;
const SUCCESS_BOOST: f64 = 5.0;
const FAILURE_PENALTY: f64 = -10.0;
const REUSE_BOOST: f64 = 3.0;
const PROMOTION_THRESHOLD: f64 = 70.0;

/// The skill learning engine.
pub struct SkillLearner {
    skills: HashMap<String, Skill>,
    pending_patterns: Vec<PatternCandidate>,
}

/// A candidate pattern before promotion to skill.
#[derive(Debug, Clone)]
struct PatternCandidate {
    pattern: String,
    tool_name: String,
    triggers: Vec<String>,
    occurrences: u32,
    success_rate: f64,
    hash: String,
}

impl SkillLearner {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            pending_patterns: Vec::new(),
        }
    }

    /// Compute SHA-256 hash for deduplication.
    pub fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Process a tool use event to detect patterns.
    pub fn process_event(&mut self, event: &ToolUseEvent) {
        if !event.success {
            // Penalize matching skills on failure
            self.penalize_matching(&event.tool_name, &event.input);
            return;
        }

        let pattern_key = format!("{}:{}", event.tool_name, self.extract_pattern(&event.input));
        let hash = Self::compute_hash(&pattern_key);

        // Check if already a skill
        if let Some(skill) = self.skills.get_mut(&hash) {
            skill.usage_count += 1;
            skill.confidence = (skill.confidence + REUSE_BOOST).min(100.0);
            return;
        }

        // Check pending patterns
        let maybe_idx = self.pending_patterns.iter().position(|c| c.hash == hash);
        if let Some(idx) = maybe_idx {
            self.pending_patterns[idx].occurrences += 1;
            let occ = self.pending_patterns[idx].occurrences;
            self.pending_patterns[idx].success_rate =
                (self.pending_patterns[idx].success_rate * (occ - 1) as f64 + 1.0) / occ as f64;

            // Check for promotion - calculate confidence inline to avoid borrow issue
            let candidate = &self.pending_patterns[idx];
            let mut confidence = BASE_CONFIDENCE;
            confidence += (candidate.occurrences as f64 - 1.0) * SUCCESS_BOOST;
            if candidate.success_rate < 0.5 {
                confidence += FAILURE_PENALTY * 2.0;
            } else if candidate.success_rate < 0.8 {
                confidence += FAILURE_PENALTY;
            }
            if candidate.triggers.len() >= 3 {
                confidence += 5.0;
            }
            let confidence = confidence.clamp(0.0, 100.0);

            if confidence >= PROMOTION_THRESHOLD {
                let skill = Skill {
                    id: hash.clone(),
                    name: format!("{}-pattern", candidate.tool_name),
                    description: format!(
                        "Auto-learned pattern for {} tool usage",
                        candidate.tool_name
                    ),
                    pattern: candidate.pattern.clone(),
                    triggers: candidate.triggers.clone(),
                    tags: vec![candidate.tool_name.clone()],
                    confidence,
                    usage_count: candidate.occurrences,
                    hash: hash.clone(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                self.skills.insert(hash, skill);
            }
        } else {
            // New candidate
            let triggers = self.extract_triggers(&event.input);
            self.pending_patterns.push(PatternCandidate {
                pattern: pattern_key,
                tool_name: event.tool_name.clone(),
                triggers,
                occurrences: 1,
                success_rate: 1.0,
                hash,
            });
        }
    }

    /// Extract a normalized pattern from tool input.
    fn extract_pattern(&self, input: &str) -> String {
        // Normalize: lowercase, collapse whitespace, remove specific values
        let lower = input.to_lowercase();
        let normalized = lower
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");
        // Truncate to prevent overly specific patterns
        let char_count: usize = normalized.chars().count();
        if char_count > 200 {
            normalized.chars().take(200).collect()
        } else {
            normalized
        }
    }

    /// Extract trigger words from input.
    fn extract_triggers(&self, input: &str) -> Vec<String> {
        let lower = input.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        // Take significant words (length > 3, not common stopwords)
        let stopwords = [
            "the", "and", "for", "with", "that", "this", "from", "have",
            "will", "been", "more", "when", "than",
        ];

        words
            .iter()
            .filter(|w| w.len() > 3 && !stopwords.contains(w))
            .take(5)
            .map(|w| w.to_string())
            .collect()
    }

    /// Penalize skills matching a failed tool use.
    fn penalize_matching(&mut self, tool_name: &str, _input: &str) {
        for skill in self.skills.values_mut() {
            if skill.tags.contains(&tool_name.to_string()) {
                skill.confidence = (skill.confidence + FAILURE_PENALTY).max(0.0);
            }
        }
    }

    /// Get all learned skills.
    pub fn skills(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// Get skills above a confidence threshold.
    pub fn confident_skills(&self, min_confidence: f64) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.confidence >= min_confidence)
            .collect()
    }

    /// Get a skill by hash.
    pub fn get_skill(&self, hash: &str) -> Option<&Skill> {
        self.skills.get(hash)
    }

    /// Find skills matching a trigger word.
    pub fn find_by_trigger(&self, trigger: &str) -> Vec<&Skill> {
        let lower = trigger.to_lowercase();
        self.skills
            .values()
            .filter(|s| s.triggers.iter().any(|t| t.contains(&lower)))
            .collect()
    }

    /// Total pending pattern count.
    pub fn pending_count(&self) -> usize {
        self.pending_patterns.len()
    }

    /// Total skill count.
    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }

    /// Import a skill directly (e.g., from persistence).
    pub fn import_skill(&mut self, skill: Skill) {
        self.skills.insert(skill.hash.clone(), skill);
    }

    /// Export all skills for persistence.
    pub fn export_skills(&self) -> Vec<Skill> {
        self.skills.values().cloned().collect()
    }
}

impl Default for SkillLearner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn success_event(tool: &str, input: &str) -> ToolUseEvent {
        ToolUseEvent {
            tool_name: tool.to_string(),
            input: input.to_string(),
            output: "success".to_string(),
            success: true,
            duration_ms: 100,
        }
    }

    fn failure_event(tool: &str, input: &str) -> ToolUseEvent {
        ToolUseEvent {
            tool_name: tool.to_string(),
            input: input.to_string(),
            output: "error".to_string(),
            success: false,
            duration_ms: 50,
        }
    }

    #[test]
    fn test_compute_hash() {
        let hash = SkillLearner::compute_hash("hello world");
        assert_eq!(hash.len(), 64);
        // Same input = same hash
        assert_eq!(hash, SkillLearner::compute_hash("hello world"));
    }

    #[test]
    fn test_different_inputs_different_hashes() {
        let h1 = SkillLearner::compute_hash("hello");
        let h2 = SkillLearner::compute_hash("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_process_first_event() {
        let mut learner = SkillLearner::new();
        learner.process_event(&success_event("bash", "cargo test"));
        assert_eq!(learner.pending_count(), 1);
        assert_eq!(learner.skill_count(), 0);
    }

    #[test]
    fn test_promotion_after_repeated_success() {
        let mut learner = SkillLearner::new();
        // Need enough occurrences to reach 70 confidence
        // Base 50 + (n-1)*5 >= 70 -> n >= 5
        for _ in 0..6 {
            learner.process_event(&success_event("bash", "cargo test"));
        }
        assert!(learner.skill_count() >= 1);
    }

    #[test]
    fn test_failure_does_not_create_pattern() {
        let mut learner = SkillLearner::new();
        learner.process_event(&failure_event("bash", "rm -rf /"));
        assert_eq!(learner.pending_count(), 0);
    }

    #[test]
    fn test_reuse_boosts_confidence() {
        let mut learner = SkillLearner::new();
        // Promote a skill first
        for _ in 0..6 {
            learner.process_event(&success_event("bash", "npm test"));
        }
        let skills: Vec<_> = learner.skills().into_iter().collect();
        assert!(!skills.is_empty());
        let initial_confidence = skills[0].confidence;

        // Reuse it
        learner.process_event(&success_event("bash", "npm test"));
        let skills: Vec<_> = learner.skills().into_iter().collect();
        assert!(skills[0].confidence > initial_confidence);
    }

    #[test]
    fn test_failure_penalizes_skills() {
        let mut learner = SkillLearner::new();
        for _ in 0..6 {
            learner.process_event(&success_event("bash", "cargo build"));
        }
        let before: f64 = learner.skills().iter().map(|s| s.confidence).sum();
        learner.process_event(&failure_event("bash", "cargo build --bad"));
        let after: f64 = learner.skills().iter().map(|s| s.confidence).sum();
        assert!(after < before);
    }

    #[test]
    fn test_find_by_trigger() {
        let mut learner = SkillLearner::new();
        let skill = Skill {
            id: "s1".into(),
            name: "test".into(),
            description: "test skill".into(),
            pattern: "test pattern".into(),
            triggers: vec!["cargo".into(), "test".into()],
            tags: vec![],
            confidence: 80.0,
            usage_count: 5,
            hash: "abc123".into(),
            created_at: "2026-01-01".into(),
        };
        learner.import_skill(skill);
        let found = learner.find_by_trigger("cargo");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_confident_skills_filter() {
        let mut learner = SkillLearner::new();
        learner.import_skill(Skill {
            id: "s1".into(), name: "high".into(), description: "".into(),
            pattern: "".into(), triggers: vec![], tags: vec![], confidence: 90.0,
            usage_count: 10, hash: "h1".into(), created_at: "".into(),
        });
        learner.import_skill(Skill {
            id: "s2".into(), name: "low".into(), description: "".into(),
            pattern: "".into(), triggers: vec![], tags: vec![], confidence: 30.0,
            usage_count: 2, hash: "h2".into(), created_at: "".into(),
        });
        assert_eq!(learner.confident_skills(50.0).len(), 1);
        assert_eq!(learner.confident_skills(20.0).len(), 2);
    }

    #[test]
    fn test_export_import() {
        let mut learner = SkillLearner::new();
        learner.import_skill(Skill {
            id: "s1".into(), name: "exported".into(), description: "".into(),
            pattern: "pat".into(), triggers: vec!["t1".into()], tags: vec![],
            confidence: 85.0, usage_count: 7, hash: "hash1".into(), created_at: "".into(),
        });
        let exported = learner.export_skills();
        assert_eq!(exported.len(), 1);

        let mut new_learner = SkillLearner::new();
        for skill in exported {
            new_learner.import_skill(skill);
        }
        assert_eq!(new_learner.skill_count(), 1);
    }

    #[test]
    fn test_get_skill_by_hash() {
        let mut learner = SkillLearner::new();
        learner.import_skill(Skill {
            id: "s1".into(), name: "findme".into(), description: "".into(),
            pattern: "".into(), triggers: vec![], tags: vec![], confidence: 80.0,
            usage_count: 5, hash: "unique_hash".into(), created_at: "".into(),
        });
        assert!(learner.get_skill("unique_hash").is_some());
        assert!(learner.get_skill("wrong").is_none());
    }

    #[test]
    fn test_extract_triggers() {
        let learner = SkillLearner::new();
        let triggers = learner.extract_triggers("cargo test --release with features");
        assert!(triggers.contains(&"cargo".to_string()));
        assert!(!triggers.contains(&"the".to_string()));
    }

    #[test]
    fn test_skill_serialization() {
        let skill = Skill {
            id: "s1".into(), name: "test".into(), description: "desc".into(),
            pattern: "pat".into(), triggers: vec!["t".into()], tags: vec!["tag".into()],
            confidence: 75.0, usage_count: 3, hash: "h".into(), created_at: "now".into(),
        };
        let json = serde_json::to_string(&skill).unwrap();
        let restored: Skill = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test");
        assert_eq!(restored.confidence, 75.0);
    }

    #[test]
    fn test_extract_pattern_unicode_no_panic() {
        let learner = SkillLearner::new();
        // Long Unicode input should not panic from byte-offset slicing
        let long_unicode = "한국어 ".repeat(100);
        let pattern = learner.extract_pattern(&long_unicode);
        assert!(pattern.chars().count() <= 200);
    }

    #[test]
    fn test_confidence_clamp() {
        let mut learner = SkillLearner::new();
        learner.import_skill(Skill {
            id: "s".into(), name: "max".into(), description: "".into(),
            pattern: "".into(), triggers: vec![], tags: vec!["bash".into()],
            confidence: 99.0, usage_count: 50, hash: "maxhash".into(), created_at: "".into(),
        });
        // Reuse many times
        for _ in 0..20 {
            let event = ToolUseEvent {
                tool_name: "bash".into(), input: "special".into(),
                output: "ok".into(), success: true, duration_ms: 10,
            };
            learner.process_event(&event);
        }
        // Confidence should not exceed 100
        for skill in learner.skills() {
            assert!(skill.confidence <= 100.0);
        }
    }
}
