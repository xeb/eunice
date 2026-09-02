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
        Provider::Abliteration => {
            // Published large-model rate; input and output have the same price.
            (5.00, 5.00)
        }
        Provider::Cerebras => {
            // Cerebras Inference pricing as reported by its live model catalog.
            if model.contains("gemma-4-31b") {
                (0.99, 1.49)
            } else {
                (0.35, 0.75) // gpt-oss-120b and the default Cerebras tier
            }
        }
        Provider::Gemini => {
            // Standard Gemini Developer API pricing as of 2026-09-01.
            if model.contains("3.1-pro") {
                (2.00, 12.00)
            } else if model.contains("3.5-flash-lite") {
                (0.30, 2.50)
            } else if model.contains("3.1-flash-lite") {
                (0.25, 1.50)
            } else if model.contains("3.7-flash") || model.contains("3.6-flash") {
                // Promotional rate through 2026-12-31.
                (0.75, 3.75)
            } else if model.contains("3.5-flash") {
                (1.50, 9.00)
            } else if model.contains("3-flash") {
                (0.50, 3.00)
            } else {
                (0.75, 3.75) // Default to the current Flash tier
            }
        }
        Provider::Anthropic => {
            // Claude API pricing as of 2026-09-01.
            if model.contains("fable") {
                (10.00, 50.00)
            } else if model.contains("opus") {
                (5.00, 25.00)
            } else if model.contains("sonnet") {
                (2.00, 10.00)
            } else if model.contains("haiku") {
                (1.00, 5.00)
            } else {
                (2.00, 10.00) // Default to Sonnet 5 pricing
            }
        }
        Provider::OpenAI | Provider::AzureOpenAI => {
            // OpenAI standard pricing as of 2026-09-01. Azure prices vary by
            // deployment and region, so these are estimates for Azure.
            if model.contains("gpt-5.6-luna") {
                (0.20, 1.20)
            } else if model.contains("gpt-5.6-terra") {
                (2.00, 12.00)
            } else if model.contains("gpt-5.3-codex") {
                (1.75, 14.00)
            } else if model == "gpt-5.6" || model.contains("gpt-5.6-sol") {
                (4.00, 20.00)
            } else {
                (4.00, 20.00) // Default to GPT-5.6 Sol pricing
            }
        }
        Provider::Ollama => {
            // Ollama is free (local)
            (0.0, 0.0)
        }
        Provider::Local => {
            // Local inference is free
            (0.0, 0.0)
        }
        Provider::Gemmad => {
            // Local daemon inference is free
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

        let cost = session.estimate_cost("gemini-3.7-flash", &Provider::Gemini);
        // $0.75/1M input + $3.75/1M output = $4.50
        assert!((cost - 4.50).abs() < 0.001);
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

        let cost = session.estimate_cost("claude-sonnet-5", &Provider::Anthropic);
        // $2.00/1M input + $10.00/1M output = $12.00
        assert!((cost - 12.0).abs() < 0.001);
    }

    #[test]
    fn test_current_model_pricing() {
        assert_eq!(get_pricing("gpt-5.6-sol", &Provider::OpenAI), (4.0, 20.0));
        assert_eq!(get_pricing("gpt-5.6-terra", &Provider::OpenAI), (2.0, 12.0));
        assert_eq!(get_pricing("gpt-5.6-luna", &Provider::OpenAI), (0.2, 1.2));
        assert_eq!(get_pricing("gpt-5.3-codex", &Provider::OpenAI), (1.75, 14.0));
        assert_eq!(get_pricing("gemma-4-31b", &Provider::Cerebras), (0.99, 1.49));
        assert_eq!(get_pricing("gpt-oss-120b", &Provider::Cerebras), (0.35, 0.75));
        assert_eq!(get_pricing("claude-fable-5-1", &Provider::Anthropic), (10.0, 50.0));
        assert_eq!(get_pricing("claude-opus-5", &Provider::Anthropic), (5.0, 25.0));
        assert_eq!(get_pricing("claude-haiku-4-5-20251001", &Provider::Anthropic), (1.0, 5.0));
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

        let summary = session.format_summary("gemini-3.7-flash", &Provider::Gemini);
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

        let cost = session.estimate_cost("gemma4", &Provider::Ollama);
        assert_eq!(cost, 0.0);
    }
}
