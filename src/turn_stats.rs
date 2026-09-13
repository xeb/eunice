//! Per-turn telemetry. Kept outside serialized model conversation history.
use crate::models::ChatCompletionResponse;

#[derive(Debug, Clone, Default)]
pub struct TurnStats {
    pub calls: u64,
    pub output_tokens: u64,
    pub known_usage: bool,
    pub generation_tokens: u64,
    pub generation_ms: f64,
    pub timed_responses: u64,
    pub responses: u64,
    pub elapsed_seconds: f64,
    pub complete: bool,
    pub outcome: String,
}

impl TurnStats {
    pub fn observe(&mut self, response: &ChatCompletionResponse) {
        self.responses += 1;
        if let Some(usage) = &response.usage {
            self.output_tokens += usage.completion_tokens;
            self.known_usage = true;
        }
        if let Some(timing) = &response.timings {
            if timing.predicted_ms.is_finite() && timing.predicted_ms > 0.0 {
                self.generation_tokens += timing.predicted_n;
                self.generation_ms += timing.predicted_ms;
                self.timed_responses += 1;
            }
        }
    }

    pub fn line(&self, session_bytes: usize, compactions: u64) -> String {
        let speed = if self.responses > 0 && self.timed_responses == self.responses {
            format!("{:.1} tok/s", self.generation_tokens as f64 * 1000.0 / self.generation_ms)
        } else if self.known_usage && self.elapsed_seconds > 0.0 {
            format!("{:.1} eff tok/s", self.output_tokens as f64 / self.elapsed_seconds)
        } else { "— tok/s".into() };
        format!("last turn · {speed} · {:.1}s · {} calls · session {} · compact {}{}",
            self.elapsed_seconds, self.calls, format_bytes(session_bytes), compactions,
            if self.outcome.is_empty() { String::new() } else { format!(" · {}", self.outcome) })
    }
}

pub fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 { format!("{bytes} B") }
    else if bytes < 1024 * 1024 { format!("{:.1} KiB", bytes as f64 / 1024.0) }
    else { format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn weights_native_speed_across_tool_rounds() {
        let mut stats = TurnStats::default();
        for (tokens, ms) in [(10, 1000), (90, 3000)] {
            stats.calls += 1;
            stats.observe(&serde_json::from_value(json!({"choices":[],"timings":{"predicted_n":tokens,"predicted_ms":ms}})).unwrap());
        }
        stats.elapsed_seconds = 10.0;
        assert_eq!(stats.line(2048, 2), "last turn · 25.0 tok/s · 10.0s · 2 calls · session 2.0 KiB · compact 2");
    }
    #[test]
    fn unavailable_and_effective_rates_are_not_mislabelled() {
        let mut stats = TurnStats::default();
        assert!(stats.line(9, 0).contains("— tok/s"));
        stats.known_usage = true; stats.output_tokens = 20; stats.elapsed_seconds = 4.0;
        assert!(stats.line(9, 0).contains("5.0 eff tok/s"));
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
    }
}
