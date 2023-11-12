//! Swarm controller implementation.

use anyhow::Result;
use futures::{future::Either, StreamExt};
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade},
    dns::tokio::Transport as TokioDnsConfig,
    identify::Event as IdentifyEvent,
    kad::{self, Event as KademliaEvent},
    mdns::Event as MdnsEvent,
    multiaddr::Protocol,
    noise, quic,
    swarm::{DialError, SwarmEvent},
    tcp::Config as TcpConfig,
    yamux, Multiaddr, PeerId, Swarm, Transport,
};
use log::{error, info};
use std::{
    net::Ipv4Addr,
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time;

use crate::{
    behaviour::{PeerNetworkBehaviour, PeerNetworkEvent},
    config::Config,
    error::CLIError,
    writer::{CSVWriter, Status},
};

pub struct Controller {
    pub config: Config,
    pub writer: CSVWriter,
    swarm: Swarm<PeerNetworkBehaviour>,
}

impl Controller {
    /// Setup a new Controller object
    pub async fn new(config: Config) -> Result<Self, CLIError> {
        // build transport layer
        let transport = Self::build_transport_layer(&config).map_err(|_| CLIError::ResourceBusy)?;

        // build network behaviour
        let behaviour = PeerNetworkBehaviour::new(&config)?;

        // initialize swarm controller
        let swarm: Swarm<PeerNetworkBehaviour> = Swarm::new(
            transport,
            behaviour,
            config.keypair.public().to_peer_id(),
            libp2p::swarm::Config::with_tokio_executor(),
        );

        let writer = CSVWriter::new()?;

        Ok(Controller {
            writer,
            config,
            swarm,
        })
    }

    /// Start the Swarm controller
    pub async fn start(mut self) -> Result<Self, CLIError> {
        // Set a listener for this swarm
        let listening_addr: Multiaddr = format!(
            "/ip4/{}/tcp/{}",
            Ipv4Addr::new(127, 0, 0, 1),
            self.config.listening_port
        )
        .parse()
        .map_err(|_| CLIError::IdentityError)?;
        self.swarm
            .listen_on(listening_addr)
            .map_err(|_| CLIError::ResourceBusy)?;

        // Dial bootstrapped nodes
        for bootstrap_addr in self.config.bootstrap_addr.iter() {
            let bootstrap_node_addr: Multiaddr = bootstrap_addr
                .parse()
                .map_err(|_| CLIError::IdentityError)?;

            let _ = self.swarm.dial(bootstrap_node_addr);
        }

        // Start event loop.
        // let _ = self.start_event_loop().await;

        Ok(self)
    }
}
