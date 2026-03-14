pub mod identity;
pub mod node;
pub mod handshake;
pub mod state;
pub mod protocol;

pub use identity::{AgentId, AgentProfile, AgentCapability};
pub use node::VertexNode;
pub use handshake::HandshakeManager;
pub use state::SwarmState;
pub use protocol::{SwarmMessage, MessageType};
