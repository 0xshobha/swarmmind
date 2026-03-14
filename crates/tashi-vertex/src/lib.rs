//! Mock Tashi Vertex SDK for Hackathon Demo
//!
//! Provides the API expected by SwarmMind.
//! Under the hood, this simulates BFT consensus using local messaging.

use std::str::FromStr;
use anyhow::Result;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct Context {}

impl Context {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
}

pub struct Socket {}

impl Socket {
    pub async fn bind(_ctx: &Context, _addr: &str) -> Result<Self> {
        Ok(Self {})
    }
}

pub struct Peers {}

impl Peers {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
    
    pub fn insert(&mut self, _addr: &str, _pubkey: &str, _defaults: ()) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct KeyPublic(String);

impl std::fmt::Display for KeyPublic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
pub struct KeySecret {
    secret: String,
    public: String,
}

impl KeySecret {
    pub fn generate() -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            secret: format!("sec_{}", id),
            public: format!("pub_{}", id),
        }
    }
    
    pub fn public(&self) -> KeyPublic {
        KeyPublic(self.public.clone())
    }
}

impl std::fmt::Display for KeySecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.secret)
    }
}

impl FromStr for KeySecret {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Just derive a deterministic public key from the secret string for local testing
        let public = s.replace("sec_", "pub_");
        Ok(Self {
            secret: s.to_string(),
            public,
        })
    }
}

#[derive(Default)]
pub struct Options {}

#[derive(Clone)]
pub struct Transaction {
    data: Vec<u8>,
}

impl Transaction {
    pub fn allocate(len: usize) -> Self {
        Self { data: vec![0; len] }
    }
    
    pub fn copy_from_slice(&mut self, data: &[u8]) {
        self.data.copy_from_slice(data);
    }
}

impl AsRef<[u8]> for Transaction {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

pub struct Event {
    txs: Vec<Transaction>,
}

impl Event {
    pub fn transactions(&self) -> impl Iterator<Item = Transaction> {
        self.txs.clone().into_iter()
    }
}

pub enum Message {
    Event(Event),
    SyncPoint(u64),
}

#[derive(Clone)]
pub struct Engine {
    // A global channel so all nodes in the same process can communicate
    // Since each CLI runs in a different process, a real demo would use UDP broadcasting
    // But for a quick mock, we will use UDP broadcasting!
    tx: broadcast::Sender<Transaction>,
    rx: std::sync::Arc<tokio::sync::Mutex<broadcast::Receiver<Transaction>>>,
}

lazy_static::lazy_static! {
    static ref GLOBAL_CHANNEL: (broadcast::Sender<Transaction>, broadcast::Receiver<Transaction>) = broadcast::channel(1024);
}

impl Engine {
    pub fn start(
        _ctx: &Context,
        _socket: Socket,
        _options: Options,
        _key: &KeySecret,
        _peers: Peers,
    ) -> Result<Self> {
        let tx = GLOBAL_CHANNEL.0.clone();
        let rx = tx.subscribe();
        
        Ok(Self {
            tx,
            rx: std::sync::Arc::new(tokio::sync::Mutex::new(rx)),
        })
    }
    
    pub fn send_transaction(&self, tx: Transaction) -> Result<()> {
        let _ = self.tx.send(tx);
        Ok(())
    }
    
    pub async fn recv_message(&self) -> Result<Option<Message>> {
        let mut rx = self.rx.lock().await;
        match rx.recv().await {
            Ok(tx) => {
                let event = Event { txs: vec![tx] };
                Ok(Some(Message::Event(event)))
            }
            Err(_) => Ok(None),
        }
    }
}
