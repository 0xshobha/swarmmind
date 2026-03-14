//! Vertex consensus node wrapper.
//!
//! Manages the lifecycle of a Tashi Vertex engine and provides
//! send/receive abstractions for SwarmMessages.

use anyhow::Result;
use tashi_vertex::{
    Context as VtxContext, Engine, KeySecret, Message, Options, Peers, Socket, Transaction,
};
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

use crate::protocol::SwarmMessage;

/// Peer connection info for bootstrapping.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub addr: String,
    pub pubkey: String,
}

/// Configuration for a Vertex node.
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub bind_addr: String,
    pub secret_key: String,
    pub peers: Vec<PeerInfo>,
}

/// A running Vertex consensus node.
pub struct VertexNode {
    engine: Engine,
    _ctx: VtxContext,
}

impl VertexNode {
    /// Start a new Vertex consensus node.
    pub async fn start(config: NodeConfig) -> Result<Self> {
        let key: KeySecret = config
            .secret_key
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid secret key: {:?}", e))?;

        // Build peer list
        let mut peers = Peers::new()
            .map_err(|e| anyhow::anyhow!("Failed to create peers: {:?}", e))?;

        // Add self to the peer list
        peers
            .insert(&config.bind_addr, &key.public().to_string(), Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to add self as peer: {:?}", e))?;

        for peer in &config.peers {
            peers
                .insert(&peer.addr, &peer.pubkey, Default::default())
                .map_err(|e| anyhow::anyhow!("Failed to add peer {}: {:?}", peer.addr, e))?;
        }

        let ctx = VtxContext::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Vertex context: {:?}", e))?;
        let socket = Socket::bind(&ctx, &config.bind_addr)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bind socket on {}: {:?}", config.bind_addr, e))?;

        let options = Options::default();
        let engine = Engine::start(&ctx, socket, options, &key, peers)
            .map_err(|e| anyhow::anyhow!("Failed to start Vertex engine: {:?}", e))?;

        info!(addr = %config.bind_addr, peers = config.peers.len(), "🚀 Vertex node started");

        Ok(Self { engine, _ctx: ctx })
    }

    /// Send a SwarmMessage through the consensus engine.
    pub fn send_message(&self, msg: &SwarmMessage) -> Result<()> {
        let data = msg.to_bytes();
        let mut tx = Transaction::allocate(data.len());
        tx.copy_from_slice(&data);
        self.engine
            .send_transaction(tx)
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {:?}", e))?;
        debug!(msg_type = ?msg.msg_type, "📤 Sent message");
        Ok(())
    }

    /// Start receiving consensus-ordered messages.
    /// Returns a channel receiver that yields SwarmMessages.
    pub fn receive_messages(&self) -> mpsc::Receiver<SwarmMessage> {
        let (tx, rx) = mpsc::channel(256);
        let engine = self.engine.clone();

        tokio::spawn(async move {
            loop {
                match engine.recv_message().await {
                    Ok(Some(message)) => match message {
                        Message::Event(event) => {
                            for transaction in event.transactions() {
                                match SwarmMessage::from_bytes(transaction.as_ref()) {
                                    Ok(msg) => {
                                        if tx.send(msg).await.is_err() {
                                            debug!("Message channel closed, stopping receiver");
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to deserialize transaction: {e}");
                                    }
                                }
                            }
                        }
                        Message::SyncPoint(_) => {
                            debug!("Received sync point");
                        }
                    },
                    Ok(None) => {
                        info!("Vertex engine stream ended");
                        break;
                    }
                    Err(e) => {
                        error!("Error receiving message: {:?}", e);
                        break;
                    }
                }
            }
        });

        rx
    }
}
