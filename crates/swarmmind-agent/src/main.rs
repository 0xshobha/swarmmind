//! SwarmMind Agent — A leaderless AI agent economy node.
//!
//! CLI entry point for running an agent in the SwarmMind decentralized
//! agent marketplace. Built on Tashi Vertex BFT consensus.

use std::sync::Arc;
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, error, Level};
use tracing_subscriber::EnvFilter;
use tokio::time::{sleep, Duration};

use vertex_core::identity::{generate_keypair, AgentCapability, AgentProfile};
use vertex_core::node::{NodeConfig, PeerInfo, VertexNode};
use agent_economy::AgentEconomy;

#[derive(Parser)]
#[command(
    name = "swarmmind-agent",
    about = "🧠 SwarmMind: Decentralized AI Agent Economy on Tashi Vertex",
    version,
    author = "SwarmMind Team"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new Ed25519 keypair for agent identity.
    GenKey,

    /// Run a SwarmMind agent node.
    Run {
        /// Socket address to bind (e.g., "127.0.0.1:9000").
        #[arg(long)]
        bind: String,

        /// Base58-encoded secret key.
        #[arg(long)]
        secret: String,

        /// Agent display name.
        #[arg(long, default_value = "agent")]
        name: String,

        /// Comma-separated capabilities (compute,data-analysis,reasoning,code-generation,image-processing,natural-language).
        #[arg(long, default_value = "compute,reasoning")]
        capabilities: String,

        /// Peer addresses (can be specified multiple times).
        #[arg(long = "peer-addr")]
        peer_addrs: Vec<String>,

        /// Peer public keys (must match peer-addr count).
        #[arg(long = "peer-pubkey")]
        peer_pubkeys: Vec<String>,

        /// Run a demo scenario after connecting.
        #[arg(long, default_value_t = false)]
        demo: bool,
    },

    /// Submit a task to the swarm.
    SubmitTask {
        /// Socket address to bind.
        #[arg(long)]
        bind: String,

        /// Base58-encoded secret key.
        #[arg(long)]
        secret: String,

        /// Task description.
        #[arg(long)]
        description: String,

        /// Required capabilities (comma-separated).
        #[arg(long, default_value = "compute")]
        capabilities: String,

        /// Task complexity (1-10).
        #[arg(long, default_value_t = 5)]
        complexity: u8,

        /// Reward amount.
        #[arg(long, default_value_t = 100)]
        reward: u64,

        /// Peer addresses.
        #[arg(long = "peer-addr")]
        peer_addrs: Vec<String>,

        /// Peer public keys.
        #[arg(long = "peer-pubkey")]
        peer_pubkeys: Vec<String>,
    },
}

fn parse_capabilities(s: &str) -> Vec<AgentCapability> {
    s.split(',')
        .map(|c| match c.trim() {
            "compute" => AgentCapability::Compute,
            "data-analysis" => AgentCapability::DataAnalysis,
            "reasoning" => AgentCapability::Reasoning,
            "image-processing" => AgentCapability::ImageProcessing,
            "code-generation" => AgentCapability::CodeGeneration,
            "natural-language" => AgentCapability::NaturalLanguage,
            other => AgentCapability::Custom(other.to_string()),
        })
        .collect()
}

fn build_peers(addrs: &[String], pubkeys: &[String]) -> Vec<PeerInfo> {
    addrs
        .iter()
        .zip(pubkeys.iter())
        .map(|(addr, pubkey)| PeerInfo {
            addr: addr.clone(),
            pubkey: pubkey.clone(),
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::GenKey => {
            let (secret, public) = generate_keypair();
            println!("╔══════════════════════════════════════════╗");
            println!("║     🔑 SwarmMind Agent Keypair           ║");
            println!("╠══════════════════════════════════════════╣");
            println!("║ Secret: {secret}");
            println!("║ Public: {public}");
            println!("╚══════════════════════════════════════════╝");
            println!();
            println!("⚠️  Keep your secret key safe! Never share it.");
            println!("📋 Use the public key as --peer-pubkey for other agents.");
        }

        Commands::Run {
            bind,
            secret,
            name,
            capabilities,
            peer_addrs,
            peer_pubkeys,
            demo,
        } => {
            if peer_addrs.len() != peer_pubkeys.len() {
                error!("Mismatch: {} peer addresses but {} peer public keys", peer_addrs.len(), peer_pubkeys.len());
                std::process::exit(1);
            }

            let caps = parse_capabilities(&capabilities);
            let peers = build_peers(&peer_addrs, &peer_pubkeys);

            println!("╔══════════════════════════════════════════════╗");
            println!("║     🧠 SwarmMind Agent Starting              ║");
            println!("╠══════════════════════════════════════════════╣");
            println!("║ Name: {name}");
            println!("║ Bind: {bind}");
            println!("║ Peers: {}", peer_addrs.len());
            println!("║ Capabilities: {capabilities}");
            println!("╚══════════════════════════════════════════════╝");

            // Parse secret to get public key for profile
            let key: tashi_vertex::KeySecret = secret.parse()
                .map_err(|e| anyhow::anyhow!("Invalid secret key: {:?}", e))?;
            let pubkey = key.public().to_string();

            let profile = AgentProfile::new(&pubkey, &name, caps);

            // Start Vertex node
            let config = NodeConfig {
                bind_addr: bind.clone(),
                secret_key: secret,
                peers,
            };

            let node = Arc::new(VertexNode::start(config).await?);
            let economy = AgentEconomy::new(profile, Arc::clone(&node));

            // Start the economy
            economy.start().await?;

            // Start message processing loop
            let mut rx = node.receive_messages();

            info!("🧠 SwarmMind agent is live! Listening for consensus messages...");

            // Optional: run demo scenario
            if demo {
                let task_mgr = Arc::clone(&economy.task_manager);
                tokio::spawn(async move {
                    // Wait for connections to establish
                    sleep(Duration::from_secs(5)).await;
                    info!("🎬 Demo: Submitting sample tasks...");

                    // Submit a series of tasks
                    let tasks = vec![
                        ("Analyze dataset for anomaly patterns", vec![AgentCapability::DataAnalysis], 4, 80),
                        ("Generate summary report from logs", vec![AgentCapability::NaturalLanguage], 3, 60),
                        ("Run ML inference on sensor batch", vec![AgentCapability::Compute, AgentCapability::Reasoning], 7, 150),
                        ("Process satellite imagery", vec![AgentCapability::ImageProcessing], 6, 120),
                        ("Compile and optimize code module", vec![AgentCapability::CodeGeneration], 5, 100),
                    ];

                    for (desc, caps, complexity, reward) in tasks {
                        match task_mgr.broadcast_task(desc, caps, complexity, reward, 30) {
                            Ok(task_id) => info!("🎬 Demo task created: {task_id}"),
                            Err(e) => error!("Demo task failed: {e}"),
                        }
                        sleep(Duration::from_secs(8)).await;
                    }

                    info!("🎬 Demo scenario complete!");
                });
            }

            // Main message loop
            while let Some(msg) = rx.recv().await {
                economy.handle_message(msg).await;
            }

            info!("Agent shutting down...");
        }

        Commands::SubmitTask {
            bind,
            secret,
            description,
            capabilities,
            complexity,
            reward,
            peer_addrs,
            peer_pubkeys,
        } => {
            let caps = parse_capabilities(&capabilities);
            let peers = build_peers(&peer_addrs, &peer_pubkeys);

            let key: tashi_vertex::KeySecret = secret.parse()
                .map_err(|e| anyhow::anyhow!("Invalid secret key: {:?}", e))?;
            let pubkey = key.public().to_string();
            let agent_id = vertex_core::AgentId::from_pubkey(&pubkey);

            let config = NodeConfig {
                bind_addr: bind,
                secret_key: secret,
                peers,
            };

            let node = Arc::new(VertexNode::start(config).await?);
            let task_mgr = agent_economy::TaskManager::new(agent_id, Arc::clone(&node));

            println!("📢 Submitting task: {description}");
            let task_id = task_mgr.broadcast_task(&description, caps, complexity, reward, 60)?;
            println!("✅ Task submitted: {task_id}");

            // Wait a bit for the transaction to propagate
            sleep(Duration::from_secs(3)).await;
            println!("Task propagated to swarm.");
        }
    }

    Ok(())
}
