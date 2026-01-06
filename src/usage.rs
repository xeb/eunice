//! Token usage tracking and cost estimation.
//!
//! Tracks token usage across a session and estimates costs based on model pricing.

use crate::models::{Provider, UsageStats};

/// Session-level usage accumulator
#[derive(Debug, Default, Clone)]
pub struct SessionUsage {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cached_tokens: u64,
    pub api_calls: u64,
}

impl SessionUsage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add usage from an API response
    pub fn add(&mut self, usage: &UsageStats) {
        self.total_input_tokens += usage.prompt_tokens;
        self.total_output_tokens += usage.completion_tokens;
        self.total_cached_tokens += usage.cached_tokens;
        self.api_calls += 1;
    }

    /// Estimate cost in USD based on model and provider
    pub fn estimate_cost(&self, model: &str, provider: &Provider) -> f64 {
        let (input_price, output_price) = get_pricing(model, provider);

        let input_cost = (self.total_input_tokens as f64 / 1_000_000.0) * input_price;
        let output_cost = (self.total_output_tokens as f64 / 1_000_000.0) * output_price;

        // Cached tokens are typically free or heavily discounted
        // For now, we assume they don't contribute to cost
        input_cost + output_cost
    }

    /// Format usage summary for display
    pub fn format_summary(&self, model: &str, provider: &Provider) -> String {
        let cost = self.estimate_cost(model, provider);

        let tokens_str = format!(
            "{} in / {} out",
            format_number(self.total_input_tokens),
            format_number(self.total_output_tokens)
        );

        let cached_str = if self.total_cached_tokens > 0 {
            format!(" ({} cached)", format_number(self.total_cached_tokens))
        } else {
            String::new()
        };

        format!(
            "Tokens: {}{}\nEstimated cost: ${:.4}",
            tokens_str,
            cached_str,
            cost
        )
    }

    /// Check if any usage was recorded
    pub fn has_usage(&self) -> bool {
        self.api_calls > 0
    }
}

/// Get pricing per 1M tokens (input, output) for a model
fn get_pricing(model: &str, provider: &Provider) -> (f64, f64) {
    match provider {
        Provider::Gemini => {
            // Gemini pricing as of 2025
            if model.contains("flash") {
                (0.075, 0.30)  // gemini-2.0-flash, gemini-2.5-flash
            } else if model.contains("pro-preview") || model.contains("3-pro") {
                (1.25, 10.00)  // gemini-3-pro-preview
            } else if model.contains("pro") {
                (1.25, 5.00)   // gemini-2.5-pro
            } else {
                (0.10, 0.40)   // Default/unknown Gemini
            }
        }
        Provider::Anthropic => {
            // Anthropic pricing as of 2025
            if model.contains("opus") {
                (15.00, 75.00)
            } else if model.contains("sonnet") {
                (3.00, 15.00)
            } else if model.contains("haiku") {
                (0.25, 1.25)
            } else {
                (3.00, 15.00)  // Default to sonnet pricing
            }
        }
        Provider::OpenAI => {
            // OpenAI pricing as of 2025
            if model.contains("gpt-4o-mini") {
                (0.15, 0.60)
            } else if model.contains("gpt-4o") {
                (2.50, 10.00)
            } else if model.contains("gpt-4-turbo") {
                (10.00, 30.00)
            } else if model.contains("gpt-3.5") {
                (0.50, 1.50)
            } else if model.contains("o1-preview") {
                (15.00, 60.00)
            } else if model.contains("o1-mini") {
                (3.00, 12.00)
            } else {
                (2.50, 10.00)  // Default to gpt-4o pricing
            }
        }
        Provider::Ollama => {
            // Ollama is free (local)
            (0.0, 0.0)
        }
    }
}

/// Format a number with commas for readability
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().rev().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_usage_add() {
        let mut session = SessionUsage::new();

        session.add(&UsageStats {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            cached_tokens: 0,
        });

        assert_eq!(session.total_input_tokens, 100);
        assert_eq!(session.total_output_tokens, 50);
        assert_eq!(session.api_calls, 1);

        session.add(&UsageStats {
            prompt_tokens: 200,
            completion_tokens: 100,
            total_tokens: 300,
            cached_tokens: 50,
        });

        assert_eq!(session.total_input_tokens, 300);
        assert_eq!(session.total_output_tokens, 150);
        assert_eq!(session.total_cached_tokens, 50);
        assert_eq!(session.api_calls, 2);
    }

    #[test]
    fn test_estimate_cost_gemini_flash() {
        let mut session = SessionUsage::new();
        session.add(&UsageStats {
            prompt_tokens: 1_000_000,
            completion_tokens: 1_000_000,
            total_tokens: 2_000_000,
            cached_tokens: 0,
        });

        let cost = session.estimate_cost("gemini-2.5-flash", &Provider::Gemini);
        // $0.075/1M input + $0.30/1M output = $0.375
        assert!((cost - 0.375).abs() < 0.001);
    }

    #[test]
    fn test_estimate_cost_anthropic_sonnet() {
        let mut session = SessionUsage::new();
        session.add(&UsageStats {
            prompt_tokens: 1_000_000,
            completion_tokens: 1_000_000,
            total_tokens: 2_000_000,
            cached_tokens: 0,
        });

        let cost = session.estimate_cost("claude-sonnet-4", &Provider::Anthropic);
        // $3.00/1M input + $15.00/1M output = $18.00
        assert!((cost - 18.0).abs() < 0.001);
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(12345678), "12,345,678");
    }

    #[test]
    fn test_format_summary() {
        let mut session = SessionUsage::new();
        session.add(&UsageStats {
            prompt_tokens: 12345,
            completion_tokens: 6789,
            total_tokens: 19134,
            cached_tokens: 1000,
        });

        let summary = session.format_summary("gemini-2.5-flash", &Provider::Gemini);
        assert!(summary.contains("12,345 in"));
        assert!(summary.contains("6,789 out"));
        assert!(summary.contains("1,000 cached"));
        assert!(summary.contains("$"));
    }

    #[test]
    fn test_ollama_free() {
        let mut session = SessionUsage::new();
        session.add(&UsageStats {
            prompt_tokens: 1_000_000,
            completion_tokens: 1_000_000,
            total_tokens: 2_000_000,
            cached_tokens: 0,
        });

        let cost = session.estimate_cost("llama3", &Provider::Ollama);
        assert_eq!(cost, 0.0);
    }
}
