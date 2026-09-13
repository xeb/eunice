//! Runtime selection and conversation-owned protocol state.
use crate::{
    models::{Message, Provider},
    openai::agents::ManagedSession,
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Arc,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Runtime {
    #[default]
    Eunice,
    OpenaiAgents,
}

impl Runtime {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eunice => "eunice",
            Self::OpenaiAgents => "openai-agents",
        }
    }

    pub fn validate(self, provider: &Provider) -> Result<()> {
        if self == Self::OpenaiAgents && *provider != Provider::OpenAI {
            bail!("--runtime openai-agents requires an OpenAI model (for example --model astra)");
        }
        Ok(())
    }
}

/// Checkpoints contain no credentials. Web storage saves state before tool effects
/// and after results; the CLI keeps it in the conversation for subsequent turns.
pub trait Checkpoint: Send + Sync {
    fn save<'a>(
        &'a self,
        state: &'a ManagedSession,
        messages: &'a [Message],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

#[derive(Default)]
pub struct Conversation {
    pub turn_stats: crate::turn_stats::TurnStats,
    pub compactions: u64,
    pub messages: Vec<Message>,
    pub managed: ManagedSession,
    pub checkpoint: Option<Arc<dyn Checkpoint>>,
}

impl Conversation {
    /// Commit a smaller context while preserving telemetry and runtime metadata.
    pub fn apply_compaction(&mut self, messages: Vec<Message>) -> anyhow::Result<bool> {
        if serde_json::to_vec(&messages)?.len() >= serde_json::to_vec(&self.messages)?.len() {
            return Ok(false);
        }
        self.messages = messages;
        self.compactions += 1;
        Ok(true)
    }

    pub fn clear(&mut self) {
        self.turn_stats = crate::turn_stats::TurnStats::default();
        self.compactions = 0;
        self.messages.clear();
        self.managed = ManagedSession::default();
    }

    pub async fn save(&self) -> Result<()> {
        if let Some(store) = &self.checkpoint {
            store.save(&self.managed, &self.messages).await?;
        }
        Ok(())
    }
}

impl From<Vec<Message>> for Conversation {
    fn from(messages: Vec<Message>) -> Self {
        Self {
            messages,
            ..Self::default()
        }
    }
}
impl Deref for Conversation {
    type Target = Vec<Message>;
    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}
impl DerefMut for Conversation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.messages
    }
}
impl Serialize for Conversation {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.messages.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_runtime_only_accepts_openai() {
        Runtime::OpenaiAgents.validate(&Provider::OpenAI).unwrap();
        for provider in [
            Provider::Anthropic,
            Provider::Gemini,
            Provider::AzureOpenAI,
            Provider::Ollama,
            Provider::Cerebras,
            Provider::Local,
            Provider::Gemmad,
            Provider::Abliteration,
        ] {
            assert!(Runtime::OpenaiAgents.validate(&provider).is_err());
            Runtime::Eunice.validate(&provider).unwrap();
        }
    }
}
