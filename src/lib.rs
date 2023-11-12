#![cfg(not(doctest))]
//! `libp2p_test_tool` is a library to interact with a libp2p network and generate a network
//! report for both unreachable and reachable peers.
//!
//! # Example
//! ```no_run
//! use log::info;
//! use libp2p_test_tool::{Config, Controller};
//!
//! let mut config: Config = Config::default();
//! let _ = Controller::new(config.clone()).await?.start().await;
//! info!("Libp2p Network Interaction complete. The results are saved in {} and {}", config.just_connected, config.unreachable_csv);
//! ```

mod behaviour;
pub mod config;
pub mod controller;
mod error;
pub mod writer;

pub use crate::{config::Config, controller::Controller, error::CLIError};
