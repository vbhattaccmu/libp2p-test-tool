//! CSV Writer module to write data to a CSV file.

use csv::Writer;
use std::{collections::BTreeMap, fs::File, path::PathBuf, sync::RwLock};

use crate::error::CLIError;

pub struct CSVWriter {
    /// Stores (PeerID)
    pub newly_connected_peer_cache: RwLock<BTreeMap<String, (String, String)>>,
    /// Stores (MultiAddress)
    pub unreachable_peer_cache: RwLock<BTreeMap<String, (String, String)>>,
}

impl CSVWriter {
    pub fn new() -> Result<Self, CLIError> {
        Ok(CSVWriter {
            newly_connected_peer_cache: RwLock::new(BTreeMap::new()),
            unreachable_peer_cache: RwLock::new(BTreeMap::new()),
        })
    }

    // `append_data_to_csv` appends records to CSV when
    // `config.operation_duration` has been covered.
    pub(crate) async fn append_data_to_csv(
        &self,
        path: PathBuf,
        status: Status,
    ) -> Result<(), CLIError> {
        let file = File::create(&path).map_err(|_| CLIError::WriterError)?;
        let mut writer = Writer::from_writer(file);
        let mut record = vec![Headers::Peer.to_string()];

        match status {
            Status::Unreachable => {
                record.push(Headers::Status.to_string());
                record.push(Headers::Timestamp.to_string());

                writer
                    .write_record(record)
                    .map_err(|_| CLIError::WriterError)?;

                for (peer, stats) in self.unreachable_peer_cache.read().unwrap().iter() {
                    writer
                        .write_record(&[peer.clone(), stats.0.clone(), stats.1.clone()])
                        .map_err(|_| CLIError::WriterError)?;
                }
            }
            Status::NewlyConnected => {
                record.push(Headers::IpAddr.to_string());
                record.push(Headers::Timestamp.to_string());

                writer
                    .write_record(record)
                    .map_err(|_| CLIError::WriterError)?;

                for (peer, stats) in self.newly_connected_peer_cache.read().unwrap().iter() {
                    writer
                        .write_record(&[peer.clone(), stats.0.clone(), stats.1.clone()])
                        .map_err(|_| CLIError::WriterError)?;
                }
            }
        };

        writer.flush().map_err(|_| CLIError::WriterError)?;

        Ok(())
    }
}

pub(crate) enum Headers {
    Peer,
    IpAddr,
    Status,
    Timestamp,
}

impl ToString for Headers {
    fn to_string(&self) -> String {
        match self {
            Headers::Peer => String::from("PeerID/MultiAddr"),
            Headers::IpAddr => String::from("IpAddr"),
            Headers::Status => String::from("Status"),
            Headers::Timestamp => String::from("Timestamp"),
        }
    }
}

pub(crate) enum Status {
    Unreachable,
    NewlyConnected,
}

impl ToString for Status {
    fn to_string(&self) -> String {
        match self {
            Status::Unreachable => String::from("Unreachable"),
            Status::NewlyConnected => String::from("NewlyConnected"),
        }
    }
}
