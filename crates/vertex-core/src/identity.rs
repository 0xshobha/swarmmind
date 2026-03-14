//! Agent identity and capability management.
//!
//! Each agent in the SwarmMind network has a unique cryptographic identity
//! (derived from its Vertex keypair) and a set of declared capabilities.

use serde::{Deserialize, Serialize};
use tashi_vertex::KeySecret;
use uuid::Uuid;

/// Capabilities an agent can advertise to the swarm.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentCapability {
    Compute,
    DataAnalysis,
    Reasoning,
    ImageProcessing,
    CodeGeneration,
    NaturalLanguage,
    Custom(String),
}

impl std::fmt::Display for AgentCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compute => write!(f, "compute"),
            Self::DataAnalysis => write!(f, "data-analysis"),
            Self::Reasoning => write!(f, "reasoning"),
            Self::ImageProcessing => write!(f, "image-processing"),
            Self::CodeGeneration => write!(f, "code-generation"),
            Self::NaturalLanguage => write!(f, "natural-language"),
            Self::Custom(s) => write!(f, "custom:{s}"),
        }
    }
}

/// Unique agent identifier derived from public key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    /// Create an AgentId from public key string.
    pub fn from_pubkey(pubkey: &str) -> Self {
        // Use first 12 chars of pubkey as short ID
        let short = if pubkey.len() > 12 {
            &pubkey[..12]
        } else {
            pubkey
        };
        Self(short.to_string())
    }

    /// Generate a random agent ID (for testing).
    pub fn random() -> Self {
        Self(Uuid::new_v4().to_string()[..12].to_string())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An agent's public profile, broadcast during the HELLO handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Unique identifier for this agent.
    pub id: AgentId,
    /// Human-readable name.
    pub name: String,
    /// What this agent can do.
    pub capabilities: Vec<AgentCapability>,
    /// Maximum concurrent tasks this agent can handle.
    pub max_concurrent_tasks: u32,
    /// Agent version string.
    pub version: String,
}

impl AgentProfile {
    pub fn new(
        pubkey: &str,
        name: impl Into<String>,
        capabilities: Vec<AgentCapability>,
    ) -> Self {
        Self {
            id: AgentId::from_pubkey(pubkey),
            name: name.into(),
            capabilities,
            max_concurrent_tasks: 3,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Check if this agent can handle a task requiring specific capabilities.
    pub fn can_handle(&self, required: &[AgentCapability]) -> bool {
        required.iter().all(|cap| self.capabilities.contains(cap))
    }
}

/// Generate a new Vertex keypair and return (secret, public) as Base58 strings.
pub fn generate_keypair() -> (String, String) {
    let secret = KeySecret::generate();
    let public = secret.public();
    (secret.to_string(), public.to_string())
}
