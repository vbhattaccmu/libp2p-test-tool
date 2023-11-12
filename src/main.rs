//! `libp2p_test_tool` is a CLI tool to interact with a libp2p network and generate a network
//! report for both unreachable and reachable peers.

use clap::Parser;
use env_logger::Env;
use log::info;

mod behaviour;
mod config;
mod controller;
mod error;
mod writer;

use crate::{config::Config, controller::Controller, error::CLIError};

#[derive(Parser, Debug)]
#[clap(
    name = "Libp2p Network Interaction CLI",
    version = "0.1.0", 
    about = "Libp2p Network Interaction CLI\n\n\
             A command line tool for interacting with a libp2p network.", 
    author = "Vikram Bhattacharjee", 
    long_about = None
)]
struct Opt {
    #[clap(subcommand)]
    argument: CliArgument,
}

#[derive(Debug, Parser)]
enum CliArgument {
    GenerateNetworkReport {
        /// A set of bootstrapped libp2p node addresses from the network you want to generate metrics from.
        /// If not supplied the tool falls back to dialing the Avail network bootstrapped node.
        #[clap(long = "bootstrap-node-addrs", display_order = 1, verbatim_doc_comment)]
        bootstrap_node_addrs: Option<String>,

        /// Path to store a CSV report on newly connected nodes in the network.
        #[clap(long = "just-connected", display_order = 2, verbatim_doc_comment)]
        just_connected: Option<String>,

        /// Path to store a CSV report on non-reachable nodes in the network.
        #[clap(long = "unreachable", display_order = 3, verbatim_doc_comment)]
        unreachable: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), CLIError> {
    ///////////////////////////
    // 1. Prepare environment.
    ///////////////////////////

    let mut config: Config = Config::default();

    env_logger::Builder::from_env(Env::default().default_filter_or(&config.log_level.clone()))
        .init();

    let CliArgument::GenerateNetworkReport {
        unreachable,
        just_connected,
        bootstrap_node_addrs,
    } = Opt::parse().argument;

    if let Some(bootstrap_addrs) = bootstrap_node_addrs {
        let bootstrap_addrs: Vec<String> = bootstrap_addrs
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        config.bootstrap_addr = bootstrap_addrs;
    };
    config.unreachable_csv = unreachable.unwrap_or(config.unreachable_csv);
    config.just_connected = just_connected.unwrap_or(config.just_connected);

    /////////////////////////////
    // 2. Start Swarm Controller.
    /////////////////////////////

    info!("Starting to interact with the chosen libp2p network...");
    let result = Controller::new(config).await?.start().await;

    if let Ok(controller) = result {
        info!(
            "Libp2p Network Interaction complete. The results are saved in {} and {}",
            &controller.config.just_connected, &controller.config.unreachable_csv
        );
    } else if let Err(e) = result {
        info!("Error: {:?}", e);
    }

    Ok(())
}
