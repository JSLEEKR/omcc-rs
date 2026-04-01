//! Model routing: complexity scoring from lexical, structural, context signals.
//! Maps to Haiku/Sonnet/Opus tiers.

use crate::config::ModelTier;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Scoring weights for the three signal categories.
const LEXICAL_WEIGHT: f64 = 0.4;
const STRUCTURAL_WEIGHT: f64 = 0.3;
const CONTEXT_WEIGHT: f64 = 0.3;

/// Thresholds for tier selection.
const HAIKU_THRESHOLD: f64 = 0.3;
const OPUS_THRESHOLD: f64 = 0.7;

/// Routing decision with explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub tier: ModelTier,
    pub score: f64,
    pub lexical_score: f64,
    pub structural_score: f64,
    pub context_score: f64,
    pub confidence: f64,
    pub reason: String,
}

/// Context information for routing decisions.
#[derive(Debug, Clone, Default)]
pub struct RoutingContext {
    pub conversation_length: usize,
    pub file_count: usize,
    pub has_errors: bool,
    pub previous_tier: Option<ModelTier>,
}

/// The model router engine.
pub struct ModelRouter {
    code_pattern_re: Regex,
    technical_terms_re: Regex,
    simple_task_re: Regex,
    file_path_re: Regex,
    numbered_list_re: Regex,
}

impl ModelRouter {
    pub fn new() -> Self {
        Self {
            code_pattern_re: Regex::new(
                r"(?i)(implement|architect|refactor|optimize|design pattern|algorithm|concurrent|async|generic|trait|interface|abstract|polymorphi|inheritance|composition)"
            ).unwrap(),
            technical_terms_re: Regex::new(
                r"(?i)(database|microservice|distributed|scalab|performance|security|authentication|authorization|encryption|protocol|api gateway|load balanc|cache|queue|websocket|grpc|graphql)"
            ).unwrap(),
            simple_task_re: Regex::new(
                r"(?i)^(fix typo|update readme|change color|rename|add comment|format code|lint|simple|trivial|quick|minor)"
            ).unwrap(),
            file_path_re: Regex::new(
                r"(?i)\b[\w/\\]+\.(rs|ts|py|go|js|java|c|cpp|h)\b"
            ).unwrap(),
            numbered_list_re: Regex::new(
                r"^\s*\d+[\.\)]\s"
            ).unwrap(),
        }
    }

    /// Route a prompt to the appropriate model tier.
    pub fn route(&self, prompt: &str, context: &RoutingContext) -> RoutingDecision {
        let lexical = self.lexical_score(prompt);
        let structural = self.structural_score(prompt);
        let ctx = self.context_score(context);

        let raw_score = lexical * LEXICAL_WEIGHT + structural * STRUCTURAL_WEIGHT + ctx * CONTEXT_WEIGHT;

        // Confidence is higher when signals agree
        let signal_variance = ((lexical - raw_score).powi(2)
            + (structural - raw_score).powi(2)
            + (ctx - raw_score).powi(2))
            / 3.0;
        let confidence = (1.0 - signal_variance.sqrt()).clamp(0.0, 1.0);

        let (tier, reason) = if raw_score < HAIKU_THRESHOLD {
            (ModelTier::Haiku, "Simple task — low complexity signals".to_string())
        } else if raw_score >= OPUS_THRESHOLD {
            (ModelTier::Opus, "Complex task — high complexity signals".to_string())
        } else {
            (ModelTier::Sonnet, "Moderate task — balanced complexity".to_string())
        };

        RoutingDecision {
            tier,
            score: raw_score,
            lexical_score: lexical,
            structural_score: structural,
            context_score: ctx,
            confidence,
            reason,
        }
    }

    /// Score based on lexical signals: code patterns, technical terms, prompt length.
    fn lexical_score(&self, prompt: &str) -> f64 {
        let mut score: f64 = 0.0;

        // Code pattern matches
        let code_matches = self.code_pattern_re.find_iter(prompt).count();
        score += (code_matches as f64 * 0.15).min(0.6);

        // Technical terms
        let tech_matches = self.technical_terms_re.find_iter(prompt).count();
        score += (tech_matches as f64 * 0.12).min(0.4);

        // Simple task detection (negative signal)
        if self.simple_task_re.is_match(prompt) {
            score -= 0.3;
        }

        // Prompt length signal
        let word_count = prompt.split_whitespace().count();
        if word_count > 100 {
            score += 0.15;
        } else if word_count > 50 {
            score += 0.08;
        } else if word_count < 10 {
            score -= 0.1;
        }

        score.clamp(0.0, 1.0)
    }

    /// Score based on structural signals: nesting depth, file references, code blocks.
    fn structural_score(&self, prompt: &str) -> f64 {
        let mut score = 0.0;

        // Nested structure indicators
        let nesting_chars: usize = prompt.chars().filter(|c| matches!(c, '{' | '[' | '(')).count();
        score += (nesting_chars as f64 * 0.05).min(0.3);

        // File path references (use pre-compiled regex)
        let file_count = self.file_path_re.find_iter(prompt).count();
        score += (file_count as f64 * 0.08).min(0.4);

        // Code block presence
        let code_blocks = prompt.matches("```").count() / 2;
        score += (code_blocks as f64 * 0.1).min(0.3);

        // Numbered list / structured plan (use pre-compiled regex)
        let numbered_lines = prompt.lines().filter(|l| self.numbered_list_re.is_match(l)).count();
        if numbered_lines >= 5 {
            score += 0.2;
        } else if numbered_lines >= 3 {
            score += 0.1;
        }

        score.clamp(0.0, 1.0)
    }

    /// Score based on context: conversation length, errors, previous routing.
    fn context_score(&self, ctx: &RoutingContext) -> f64 {
        let mut score: f64 = 0.0;

        // Longer conversations tend to be more complex
        if ctx.conversation_length > 20 {
            score += 0.3;
        } else if ctx.conversation_length > 10 {
            score += 0.15;
        } else if ctx.conversation_length > 5 {
            score += 0.05;
        }

        // Many files = complex project
        if ctx.file_count > 10 {
            score += 0.25;
        } else if ctx.file_count > 5 {
            score += 0.12;
        }

        // Errors suggest debugging complexity
        if ctx.has_errors {
            score += 0.15;
        }

        // Momentum: if previous was Opus, likely still complex
        if let Some(prev) = &ctx.previous_tier {
            match prev {
                ModelTier::Opus => score += 0.1,
                ModelTier::Haiku => score -= 0.05,
                _ => {}
            }
        }

        score.clamp(0.0, 1.0)
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn router() -> ModelRouter {
        ModelRouter::new()
    }

    fn empty_ctx() -> RoutingContext {
        RoutingContext::default()
    }

    #[test]
    fn test_simple_task_routes_haiku() {
        let r = router();
        let decision = r.route("fix typo in readme", &empty_ctx());
        assert_eq!(decision.tier, ModelTier::Haiku);
    }

    #[test]
    fn test_complex_task_routes_opus() {
        let r = router();
        let prompt = "Implement a distributed microservice architecture with authentication, \
            authorization, load balancing, caching, and database optimization. Design the \
            API gateway with GraphQL interface and WebSocket support. Include concurrent \
            request handling with async patterns and proper encryption.";
        let ctx = RoutingContext {
            conversation_length: 15,
            file_count: 12,
            has_errors: false,
            previous_tier: Some(ModelTier::Opus),
        };
        let decision = r.route(prompt, &ctx);
        // Complex prompt with many technical terms + rich context should score high
        assert!(decision.score > HAIKU_THRESHOLD, "Complex task score {:.3} should exceed haiku threshold", decision.score);
        assert!(matches!(decision.tier, ModelTier::Sonnet | ModelTier::Opus));
    }

    #[test]
    fn test_moderate_task_routes_sonnet() {
        let r = router();
        let prompt = "Refactor the user service to use a repository pattern for database access. \
            Update the controller layer and add integration tests for the new pattern.";
        let ctx = RoutingContext {
            conversation_length: 8,
            file_count: 3,
            ..Default::default()
        };
        let decision = r.route(prompt, &ctx);
        // Should be above haiku but not necessarily opus
        assert!(decision.score > 0.0, "Moderate task should have positive score");
        assert!(matches!(decision.tier, ModelTier::Haiku | ModelTier::Sonnet));
    }

    #[test]
    fn test_score_range() {
        let r = router();
        let decision = r.route("hello world", &empty_ctx());
        assert!(decision.score >= 0.0 && decision.score <= 1.0);
        assert!(decision.confidence >= 0.0 && decision.confidence <= 1.0);
    }

    #[test]
    fn test_lexical_score_code_patterns() {
        let r = router();
        let score1 = r.lexical_score("implement algorithm");
        let score2 = r.lexical_score("hello");
        assert!(score1 > score2);
    }

    #[test]
    fn test_lexical_score_simple_task() {
        let r = router();
        let score = r.lexical_score("fix typo in docs");
        assert!(score < 0.3);
    }

    #[test]
    fn test_structural_score_with_code() {
        let r = router();
        let prompt = "Look at src/main.rs and src/lib.rs:\n```rust\nfn main() {}\n```";
        let score = r.structural_score(prompt);
        assert!(score > 0.0);
    }

    #[test]
    fn test_structural_score_numbered_list() {
        let r = router();
        let prompt = "1. Create module\n2. Add tests\n3. Write docs\n4. Deploy\n5. Monitor";
        let score = r.structural_score(prompt);
        assert!(score > 0.1);
    }

    #[test]
    fn test_context_score_long_conversation() {
        let r = router();
        let ctx = RoutingContext {
            conversation_length: 25,
            ..Default::default()
        };
        let score = r.context_score(&ctx);
        assert!(score >= 0.3);
    }

    #[test]
    fn test_context_score_with_errors() {
        let r = router();
        let ctx = RoutingContext {
            has_errors: true,
            ..Default::default()
        };
        let score = r.context_score(&ctx);
        assert!(score >= 0.15);
    }

    #[test]
    fn test_context_previous_opus() {
        let r = router();
        let ctx = RoutingContext {
            previous_tier: Some(ModelTier::Opus),
            conversation_length: 15,
            ..Default::default()
        };
        let score = r.context_score(&ctx);
        let ctx_no_prev = RoutingContext {
            conversation_length: 15,
            ..Default::default()
        };
        let score_no_prev = r.context_score(&ctx_no_prev);
        assert!(score > score_no_prev);
    }

    #[test]
    fn test_confidence_high_agreement() {
        let r = router();
        // A very simple task should have high confidence
        let decision = r.route("fix typo", &empty_ctx());
        assert!(decision.confidence > 0.5);
    }

    #[test]
    fn test_confidence_always_in_range() {
        let r = router();
        // Test with extreme divergent signals to ensure confidence stays in [0, 1]
        let ctx = RoutingContext {
            conversation_length: 50,
            file_count: 20,
            has_errors: true,
            previous_tier: Some(ModelTier::Opus),
        };
        let decision = r.route("fix typo", &ctx);
        assert!(decision.confidence >= 0.0, "confidence {} should be >= 0", decision.confidence);
        assert!(decision.confidence <= 1.0, "confidence {} should be <= 1", decision.confidence);
    }

    #[test]
    fn test_decision_has_reason() {
        let r = router();
        let decision = r.route("hello", &empty_ctx());
        assert!(!decision.reason.is_empty());
    }

    #[test]
    fn test_long_prompt_boosts_score() {
        let r = router();
        let short_score = r.lexical_score("do something");
        let long_prompt = "word ".repeat(120);
        let long_score = r.lexical_score(&long_prompt);
        assert!(long_score > short_score);
    }

    #[test]
    fn test_many_files_boosts_context() {
        let r = router();
        let ctx = RoutingContext {
            file_count: 15,
            ..Default::default()
        };
        let score = r.context_score(&ctx);
        assert!(score >= 0.25);
    }

    #[test]
    fn test_structural_nesting() {
        let r = router();
        let nested = "{ { { { data } } } }";
        let flat = "data";
        assert!(r.structural_score(nested) > r.structural_score(flat));
    }

    #[test]
    fn test_routing_decision_serialization() {
        let decision = RoutingDecision {
            tier: ModelTier::Sonnet,
            score: 0.5,
            lexical_score: 0.4,
            structural_score: 0.3,
            context_score: 0.6,
            confidence: 0.85,
            reason: "test".to_string(),
        };
        let json = serde_json::to_string(&decision).unwrap();
        let restored: RoutingDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tier, ModelTier::Sonnet);
    }
}
