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
        let _ = self.start_event_loop().await;

        Ok(self)
    }

    /// The main event handler for swarm. Works on Identify, Kad and MDNS
    /// 1. Identify: In this context, identifies peers with whom connection has been established.
    ///              Also asks the peer node to discover closest peers on DHT.
    /// 2. Kademlia: In this context, the peer discovery protocol where a dialled peer searches
    ///              for peers closest to it when the `get_closest_peer` query is triggered.
    /// 3. Mdns: In this context, mdns facililates/speeds up discovery of initial set of
    ///          nodes in the local network even before `kad` protocol is triggered.
    ///          The mdns protocol allows the tool to start interacting with the network with little to no
    ///          prerequisite information of bootstrapped peers.
    async fn start_event_loop(&mut self) -> Result<(), CLIError> {
        let current_instant = Instant::now();
        let mut bootstrap_interval =
            time::interval(Duration::from_secs(self.config.bootstrap_period));

        loop {
            tokio::select! {
                event = self.swarm.next() => {
                    match event.expect("Stream should be infinite.") {
                        SwarmEvent::Behaviour(PeerNetworkEvent::Identify(event)) => match event {
                            IdentifyEvent::Received { peer_id, info } => {
                                info!("[Identify]: Received identify: Peer ID: {} Listen addrs: {:?} {:?}", peer_id, info.listen_addrs, info.observed_addr);
                                info.listen_addrs
                                    .into_iter()
                                    .filter(|multi_addr| multi_addr.to_string().contains(Protocol::P2p(peer_id).tag()))
                                    .for_each(|multi_addr| {
                                        info!("[Task 3]: Resolve IP for newly connected peers.");
                                        if let Ok(ip) = Self::get_peer_ip(&info.observed_addr) {
                                            info!("[Task 2]: Log newly connected peers.");
                                            self.writer.newly_connected_peer_cache.write().unwrap().entry(peer_id.to_string())
                                                .or_insert((ip, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string()));
                                        } else {
                                            error!("[Swarm]: Failed to resolve peer IP.")
                                        }
                                        // Add the peer to DHT
                                        self.swarm.behaviour_mut().add_address(&peer_id, multi_addr);
                                        // Ask peer to discover more peers
                                        self.swarm.behaviour_mut().get_closest_peers(peer_id);
                                    });
                            }
                            IdentifyEvent::Sent { peer_id } => {
                                info!("[Identify]: Sent peer_id for identify {:?}", peer_id);
                            }
                            IdentifyEvent::Error { peer_id, error } => {
                                info!("[Identify]: Error peer_id {:?} error {:?}", peer_id, error);
                            }
                            _ => {}
                        }
                        SwarmEvent::Behaviour(PeerNetworkEvent::Mdns(event)) => match event {
                            MdnsEvent::Discovered(addrs_list) => {
                                info!("[Mdns]: Discovered peer: {:?}", addrs_list);
                                addrs_list
                                    .into_iter()
                                    .filter(|a| a.1.to_string().contains(Protocol::P2p(a.0).tag()))
                                    .for_each(|a| {
                                        info!(
                                            "[Mdns]: Discovered Peer: {} {}",
                                            a.0.to_string(),
                                            a.1.to_string()
                                        );
                                        // Peers discovered! Time to dial them
                                        let _ = self.swarm.dial(a.1.clone());
                                    });
                            }
                            MdnsEvent::Expired(addrs_list) => {
                                info!("[Mdns]: Expired list {:?}", addrs_list);
                            }
                        }
                        SwarmEvent::Behaviour(PeerNetworkEvent::Kad(event)) => match event {
                            KademliaEvent::OutboundQueryProgressed {
                                result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                                ..
                            } => {
                                if ok.peers.is_empty() {
                                    info!("[Kad]: Query finished with no closest peers.")
                                } else {
                                    info!("[Kad]: Query finished with closest peers: {:#?}", ok.peers);
                                }
                                for peer in ok.peers {
                                    // Peers discovered! Time to dial them
                                    let _ = self.swarm.dial(peer);
                                }
                            }
                            KademliaEvent::OutboundQueryProgressed {
                                result:
                                    kad::QueryResult::GetClosestPeers(Err(kad::GetClosestPeersError::Timeout {
                                        ..
                                    })),
                                ..
                            } => {
                                info!("[Kad]: Query for closest peers timed out")
                            }
                            _ => {}
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            info!("[Swarm]: Connection Established {}", peer_id);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            info!("[Swarm]: Connection Closed  {}", peer_id);
                        }
                        SwarmEvent::IncomingConnection { local_addr, .. } => {
                            info!("[Swarm]: IncomingConnection {}", local_addr);
                        }
                        SwarmEvent::OutgoingConnectionError { peer_id, error, ..} => {
                            info!("[Swarm]: OutgoingConnectionError {:?}, {:?}", peer_id, error);
                            match error {
                                DialError::Transport(addrs) => {
                                    for addr in addrs.iter() {
                                        info!("[Task 1]: Log MultiAddress if not reachable {}.", addr.0);
                                        self.writer.unreachable_peer_cache.write().unwrap().entry(addr.0.to_string())
                                            .or_insert((
                                                Status::Unreachable.to_string(),
                                                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string()
                                            ));
                                    }
                                }
                                _ => {}
                            }
                        }
                        e => info!("[Swarm]: Event {:?}", e),
                    }
                }
                _ = bootstrap_interval.tick() => self.swarm.behaviour_mut().bootstrap(),
            }

            // If time elapsed crosses max allowed operation time, write to CSV and break
            if current_instant.elapsed() > Duration::from_secs(self.config.operation_duration) {
                info!("[CSVWriter]: Writing newly connected peers to CSV.");
                let _ = self
                    .writer
                    .append_data_to_csv(
                        PathBuf::from_str(&self.config.just_connected).unwrap(),
                        Status::NewlyConnected,
                    )
                    .await;

                info!("[CSVWriter]: Writing unreachable peers to CSV");
                let _ = self
                    .writer
                    .append_data_to_csv(
                        PathBuf::from_str(&self.config.unreachable_csv).unwrap(),
                        Status::Unreachable,
                    )
                    .await;

                break;
            }
        }

        Ok(())
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

    /// A utility helper to resolve Peer IP address via `multiaddr::Protocol`
    fn get_peer_ip(multi_addr: &Multiaddr) -> Result<String, CLIError> {
        if let Some(protocol) = multi_addr.into_iter().next() {
            match protocol {
                Protocol::Ip4(ip) => return Ok(ip.to_string()),
                Protocol::Ip6(ip) => return Ok(ip.to_string()),
                Protocol::Dns(addr) => return Ok(addr.to_string()),
                _ => {
                    return Err(CLIError::IPResolutionError);
                }
            }
        }

        Err(CLIError::IPResolutionError)
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        let cache = self.writer.newly_connected_peer_cache.read().unwrap();

        info!("Cleaning up network artifacts....");
        for (peer, _) in cache.iter() {
            self.swarm
                .behaviour_mut()
                .remove_peer(PeerId::from_str(peer).unwrap());
        }
    }
}
