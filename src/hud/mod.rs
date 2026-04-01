//! HUD statusline: format status info (mode, model, agent, progress) for terminal display.

use serde::{Deserialize, Serialize};

/// HUD state containing all displayable information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HudState {
    pub mode: Option<String>,
    pub model: Option<String>,
    pub agent: Option<String>,
    pub stage: Option<String>,
    pub progress: Option<u32>,
    pub git_branch: Option<String>,
    pub context_used: Option<f64>,
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    pub rate_limit_remaining: Option<u32>,
    pub active_agents: Vec<String>,
    pub errors: Vec<String>,
}

/// ANSI color codes for terminal output.
pub struct Colors;

impl Colors {
    pub const RESET: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const DIM: &'static str = "\x1b[2m";
    pub const RED: &'static str = "\x1b[31m";
    pub const GREEN: &'static str = "\x1b[32m";
    pub const YELLOW: &'static str = "\x1b[33m";
    pub const BLUE: &'static str = "\x1b[34m";
    pub const MAGENTA: &'static str = "\x1b[35m";
    pub const CYAN: &'static str = "\x1b[36m";
    pub const WHITE: &'static str = "\x1b[37m";
}

/// The HUD renderer.
pub struct HudRenderer {
    max_width: usize,
    use_colors: bool,
}

impl HudRenderer {
    pub fn new(max_width: usize, use_colors: bool) -> Self {
        Self {
            max_width,
            use_colors,
        }
    }

    /// Render the full statusline from HUD state.
    pub fn render(&self, state: &HudState) -> String {
        let mut parts = Vec::new();

        // Mode indicator
        if let Some(mode) = &state.mode {
            parts.push(self.render_mode(mode));
        }

        // Model
        if let Some(model) = &state.model {
            parts.push(self.render_model(model));
        }

        // Agent
        if let Some(agent) = &state.agent {
            parts.push(self.render_agent(agent));
        }

        // Stage + progress
        if let Some(stage) = &state.stage {
            let progress_str = state
                .progress
                .map(|p| format!(" {}%", p))
                .unwrap_or_default();
            parts.push(self.colorize(&format!("[{}{}]", stage, progress_str), Colors::CYAN));
        }

        // Git branch
        if let Some(branch) = &state.git_branch {
            parts.push(self.colorize(&format!("git:{}", branch), Colors::DIM));
        }

        // Context usage
        if let Some(ctx) = state.context_used {
            let color = if ctx > 0.8 {
                Colors::RED
            } else if ctx > 0.5 {
                Colors::YELLOW
            } else {
                Colors::GREEN
            };
            parts.push(self.colorize(&format!("ctx:{:.0}%", ctx * 100.0), color));
        }

        // Token counts
        if let (Some(tin), Some(tout)) = (state.tokens_in, state.tokens_out) {
            let fmt_tok = |t: u64| -> String {
                if t >= 1000 {
                    format!("{}k", t / 1000)
                } else {
                    t.to_string()
                }
            };
            parts.push(self.colorize(
                &format!("tok:{}/{}", fmt_tok(tin), fmt_tok(tout)),
                Colors::DIM,
            ));
        }

        // Rate limit
        if let Some(remaining) = state.rate_limit_remaining {
            let color = if remaining < 10 {
                Colors::RED
            } else if remaining < 50 {
                Colors::YELLOW
            } else {
                Colors::GREEN
            };
            parts.push(self.colorize(&format!("rate:{}", remaining), color));
        }

        // Active agents
        if !state.active_agents.is_empty() {
            let agents_str = state.active_agents.join(",");
            parts.push(self.colorize(&format!("agents:[{}]", agents_str), Colors::MAGENTA));
        }

        // Errors
        if !state.errors.is_empty() {
            parts.push(self.colorize(
                &format!("ERR:{}", state.errors.len()),
                Colors::RED,
            ));
        }

        let line = parts.join(" | ");
        self.truncate_to_width(&line)
    }

    /// Render a compact statusline (minimal info).
    pub fn render_compact(&self, state: &HudState) -> String {
        let mut parts = Vec::new();

        if let Some(mode) = &state.mode {
            parts.push(self.render_mode(mode));
        }
        if let Some(stage) = &state.stage {
            parts.push(self.colorize(stage, Colors::CYAN));
        }
        if let Some(progress) = state.progress {
            parts.push(format!("{}%", progress));
        }

        parts.join(" ")
    }

    /// Render a progress bar.
    pub fn render_progress_bar(&self, progress: u32, width: usize) -> String {
        let clamped = progress.min(100) as usize;
        let filled = (clamped * width) / 100;
        let empty = width.saturating_sub(filled);
        let bar = format!(
            "[{}{}] {}%",
            "=".repeat(filled),
            " ".repeat(empty),
            clamped
        );
        self.colorize(&bar, Colors::GREEN)
    }

    fn render_mode(&self, mode: &str) -> String {
        let (icon, color) = match mode {
            "autopilot" => ("AP", Colors::GREEN),
            "ralph" => ("RL", Colors::YELLOW),
            "ultrawork" => ("UW", Colors::MAGENTA),
            "ultrathink" => ("UT", Colors::CYAN),
            "compact" => ("CM", Colors::DIM),
            _ => ("??", Colors::WHITE),
        };
        self.colorize(&format!("[{}]", icon), color)
    }

    fn render_model(&self, model: &str) -> String {
        let color = if model.contains("opus") {
            Colors::MAGENTA
        } else if model.contains("sonnet") {
            Colors::BLUE
        } else {
            Colors::GREEN
        };
        self.colorize(model, color)
    }

    fn render_agent(&self, agent: &str) -> String {
        self.colorize(&format!("@{}", agent), Colors::CYAN)
    }

    fn colorize(&self, text: &str, color: &str) -> String {
        if self.use_colors {
            format!("{}{}{}", color, text, Colors::RESET)
        } else {
            text.to_string()
        }
    }

    fn truncate_to_width(&self, line: &str) -> String {
        // Strip ANSI codes for length calculation
        let plain = strip_ansi(line);
        if plain.len() <= self.max_width || self.max_width < 4 {
            line.to_string()
        } else {
            // Truncate the plain text, but we need to be careful with ANSI codes
            // Simple approach: truncate and add reset
            let mut result = String::new();
            let mut plain_len = 0;
            let mut in_escape = false;
            let cutoff = self.max_width.saturating_sub(3);

            for ch in line.chars() {
                if ch == '\x1b' {
                    in_escape = true;
                    result.push(ch);
                } else if in_escape {
                    result.push(ch);
                    if ch.is_ascii_alphabetic() {
                        in_escape = false;
                    }
                } else {
                    if plain_len >= cutoff {
                        result.push_str("...");
                        if self.use_colors {
                            result.push_str(Colors::RESET);
                        }
                        break;
                    }
                    result.push(ch);
                    plain_len += 1;
                }
            }
            result
        }
    }
}

impl Default for HudRenderer {
    fn default() -> Self {
        Self::new(120, true)
    }
}

/// Strip ANSI escape codes from a string.
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    for ch in s.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_renderer() -> HudRenderer {
        HudRenderer::new(120, false)
    }

    fn color_renderer() -> HudRenderer {
        HudRenderer::new(120, true)
    }

    fn full_state() -> HudState {
        HudState {
            mode: Some("autopilot".to_string()),
            model: Some("sonnet".to_string()),
            agent: Some("executor".to_string()),
            stage: Some("EXECUTING".to_string()),
            progress: Some(40),
            git_branch: Some("main".to_string()),
            context_used: Some(0.3),
            tokens_in: Some(15000),
            tokens_out: Some(5000),
            rate_limit_remaining: Some(80),
            active_agents: vec!["executor".to_string(), "reviewer".to_string()],
            errors: vec![],
        }
    }

    #[test]
    fn test_render_empty_state() {
        let r = plain_renderer();
        let output = r.render(&HudState::default());
        assert!(output.is_empty() || output.trim().is_empty());
    }

    #[test]
    fn test_render_full_state() {
        let r = plain_renderer();
        let output = r.render(&full_state());
        assert!(output.contains("[AP]"));
        assert!(output.contains("sonnet"));
        assert!(output.contains("@executor"));
        assert!(output.contains("EXECUTING"));
        assert!(output.contains("40%"));
    }

    #[test]
    fn test_render_with_colors() {
        let r = color_renderer();
        let output = r.render(&full_state());
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_render_mode_icons() {
        let r = plain_renderer();
        let mut state = HudState::default();

        state.mode = Some("autopilot".to_string());
        assert!(r.render(&state).contains("[AP]"));

        state.mode = Some("ralph".to_string());
        assert!(r.render(&state).contains("[RL]"));

        state.mode = Some("ultrawork".to_string());
        assert!(r.render(&state).contains("[UW]"));
    }

    #[test]
    fn test_render_compact() {
        let r = plain_renderer();
        let state = HudState {
            mode: Some("autopilot".to_string()),
            stage: Some("PLANNING".to_string()),
            progress: Some(10),
            ..Default::default()
        };
        let output = r.render_compact(&state);
        assert!(output.contains("[AP]"));
        assert!(output.contains("PLANNING"));
        assert!(output.contains("10%"));
    }

    #[test]
    fn test_render_progress_bar() {
        let r = plain_renderer();
        let bar = r.render_progress_bar(50, 20);
        assert!(bar.contains("=========="));
        assert!(bar.contains("50%"));
    }

    #[test]
    fn test_render_progress_bar_empty() {
        let r = plain_renderer();
        let bar = r.render_progress_bar(0, 10);
        assert!(bar.contains("0%"));
    }

    #[test]
    fn test_render_progress_bar_full() {
        let r = plain_renderer();
        let bar = r.render_progress_bar(100, 10);
        assert!(bar.contains("100%"));
        assert!(bar.contains("=========="));
    }

    #[test]
    fn test_context_color_thresholds() {
        let r = color_renderer();

        // Low usage = green
        let state = HudState {
            context_used: Some(0.2),
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains(Colors::GREEN));

        // High usage = red
        let state = HudState {
            context_used: Some(0.9),
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains(Colors::RED));
    }

    #[test]
    fn test_rate_limit_color() {
        let r = color_renderer();
        let state = HudState {
            rate_limit_remaining: Some(5),
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains(Colors::RED));
    }

    #[test]
    fn test_error_display() {
        let r = plain_renderer();
        let state = HudState {
            errors: vec!["err1".to_string(), "err2".to_string()],
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains("ERR:2"));
    }

    #[test]
    fn test_strip_ansi() {
        let input = "\x1b[31mhello\x1b[0m world";
        assert_eq!(strip_ansi(input), "hello world");
    }

    #[test]
    fn test_strip_ansi_no_codes() {
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn test_truncate_to_width() {
        let r = HudRenderer::new(20, false);
        let long = "a".repeat(30);
        let truncated = r.truncate_to_width(&long);
        assert!(truncated.len() <= 20);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_no_truncate_short() {
        let r = HudRenderer::new(100, false);
        let short = "hello";
        assert_eq!(r.truncate_to_width(short), "hello");
    }

    #[test]
    fn test_hud_state_serialization() {
        let state = full_state();
        let json = serde_json::to_string(&state).unwrap();
        let restored: HudState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.mode, Some("autopilot".to_string()));
        assert_eq!(restored.progress, Some(40));
    }

    #[test]
    fn test_active_agents_display() {
        let r = plain_renderer();
        let state = HudState {
            active_agents: vec!["a1".to_string(), "a2".to_string()],
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains("a1,a2"));
    }

    #[test]
    fn test_token_display() {
        let r = plain_renderer();
        let state = HudState {
            tokens_in: Some(25000),
            tokens_out: Some(8000),
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains("tok:25k/8k"));
    }

    #[test]
    fn test_token_display_small() {
        let r = plain_renderer();
        let state = HudState {
            tokens_in: Some(500),
            tokens_out: Some(200),
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains("tok:500/200"));
    }

    #[test]
    fn test_git_branch_display() {
        let r = plain_renderer();
        let state = HudState {
            git_branch: Some("feature/cool".to_string()),
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains("git:feature/cool"));
    }

    #[test]
    fn test_model_display() {
        let r = plain_renderer();
        let state = HudState {
            model: Some("opus".to_string()),
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains("opus"));
    }

    #[test]
    fn test_progress_bar_over_100() {
        let r = plain_renderer();
        // Should not overflow or produce extra-wide bar
        let bar = r.render_progress_bar(150, 10);
        assert!(bar.contains("100%"));
    }

    #[test]
    fn test_truncate_very_small_width() {
        // max_width < 3 should not panic from underflow
        let r = HudRenderer::new(2, false);
        let long = "a".repeat(30);
        let _result = r.truncate_to_width(&long);
        // Just verifying no panic
    }

    #[test]
    fn test_render_unicode_mode() {
        let r = plain_renderer();
        let state = HudState {
            mode: Some("unknown_mode".to_string()),
            ..Default::default()
        };
        let output = r.render(&state);
        assert!(output.contains("[??]"));
    }
}
