//! Network Behaviour definitions for swarm controller.

use libp2p::{
    identify::{Behaviour as Identify, Config as IdentifyConfig, Event as IdentifyEvent},
    kad::{
        store::MemoryStore, Behaviour as Kademlia, Config as KademliaConfig, Event as KademliaEvent,
    },
    mdns::{tokio::Behaviour as Mdns, Config as MdnsConfig, Event as MdnsEvent},
    swarm::NetworkBehaviour,
    Multiaddr, PeerId,
};

use crate::{config::Config, error::CLIError};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "PeerNetworkEvent")]
pub(crate) struct PeerNetworkBehaviour {
    mdns: Mdns,
    identify: Identify,
    kad: Kademlia<MemoryStore>,
}

impl PeerNetworkBehaviour {
    pub fn new(config: &Config) -> Result<Self, CLIError> {
        let local_peer_id = PeerId::from(config.keypair.public());

        // Configure identify
        let identify = Identify::new(IdentifyConfig::new(
            config.protocol.clone(),
            config.keypair.public(),
        ));

        // Configure mdns
        let mdns =
            Mdns::new(MdnsConfig::default(), local_peer_id).map_err(|_| CLIError::ResourceBusy)?;

        // Configure kad
        let mut kad = Kademlia::<MemoryStore>::with_config(
            local_peer_id,
            MemoryStore::new(local_peer_id),
            KademliaConfig::default(),
        );

        // Run kad in client mode
        kad.set_mode(Some(libp2p::kad::Mode::Client));

        Ok(Self {
            mdns,
            kad,
            identify,
        })
    }
}

#[derive(Debug)]
pub(crate) enum PeerNetworkEvent {
    Kad(KademliaEvent),
    Mdns(MdnsEvent),
    Identify(IdentifyEvent),
}

impl From<IdentifyEvent> for PeerNetworkEvent {
    fn from(event: IdentifyEvent) -> Self {
        Self::Identify(event)
    }
}

impl From<MdnsEvent> for PeerNetworkEvent {
    fn from(event: MdnsEvent) -> Self {
        Self::Mdns(event)
    }
}

impl From<KademliaEvent> for PeerNetworkEvent {
    fn from(event: KademliaEvent) -> Self {
        Self::Kad(event)
    }
}

impl PeerNetworkBehaviour {
    /// Remove a peer from DHT
    pub fn remove_peer(&mut self, peer_id: PeerId) {
        self.kad.remove_peer(&peer_id);
    }

    /// Bootstrap DHT
    pub fn bootstrap(&mut self) {
        let _ = self.kad.bootstrap();
    }

    /// Add address to DHT
    pub fn add_address(&mut self, peer: &PeerId, address: Multiaddr) {
        self.kad.add_address(peer, address);
    }

    /// Query the network with a PeerId so as to discover
    /// other peers in the network
    pub fn get_closest_peers(&mut self, peer_id: PeerId) {
        self.kad.get_closest_peers(peer_id);
    }
}
