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

    /// The transport layer builder for swarm. Currently supports only tcp and quic.
    fn build_transport_layer(config: &Config) -> std::io::Result<Boxed<(PeerId, StreamMuxerBox)>> {
        let tcp_transport = libp2p::tcp::tokio::Transport::new(TcpConfig::new().nodelay(true))
            .upgrade(upgrade::Version::V1Lazy)
            .authenticate(
                noise::Config::new(&config.keypair).expect("signing libp2p-noise static keypair"),
            )
            .multiplex(yamux::Config::default())
            .timeout(std::time::Duration::from_secs(config.timeout))
            .boxed();

        let quic_transport = quic::tokio::Transport::new(quic::Config::new(&config.keypair));

        info!("[Task 3]: Create Dns config to Resolve IP.");
        let transport = TokioDnsConfig::system(libp2p::core::transport::OrTransport::new(
            quic_transport,
            tcp_transport,
        ))
        .unwrap()
        .map(|either_output, _| match either_output {
            Either::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
            Either::Right((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
        })
        .boxed();

        Ok(transport)
    }
}
