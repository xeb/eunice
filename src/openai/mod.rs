pub mod agents;
pub mod responses;
pub mod sse;

pub fn is_astra(model: &str) -> bool {
    model == "astra" || model == "gpt-6-astra" || model.starts_with("gpt-6-astra-")
}
