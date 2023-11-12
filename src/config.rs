//! Config module for the tool.

use libp2p::identity::{self, Keypair};

#[derive(Clone)]
pub struct Config {
    /// Event timeout (in s)
    pub timeout: u64,
    /// Protocol name
    pub protocol: String,
    /// Swarm keypair
    pub keypair: Keypair,
    /// Swarm listening port
    pub listening_port: u16,
    /// DHT Bootstrap period (in s)
    pub bootstrap_period: u64,
    /// CSV file input for unreachable peers
    pub unreachable_csv: String,
    /// CSV file input for newly connected peers
    pub just_connected: String,
    /// Relay address for dialing  
    pub bootstrap_addr: Vec<String>,
    /// Log Level Setting
    pub log_level: String,
    /// Time duration till the tool operates (in s)
    pub operation_duration: u64,
}

/// For convenience, default values are predefined
impl Default for Config {
    fn default() -> Self {
        Config {
            timeout: 20,
            bootstrap_period: 5,
            listening_port: 7072,
            operation_duration: 181,
            log_level: String::from("info"),
            keypair: identity::Keypair::generate_ed25519(),
            just_connected: String::from("/home/newly_connected.csv"),
            protocol: String::from("/light-client-test/1.0.0"),
            unreachable_csv: String::from("/home/unreachable.csv"),
            bootstrap_addr: vec![String::from("/ip4/172.16.3.2/udp/39000/quic-v1")],
        }
    }
}
