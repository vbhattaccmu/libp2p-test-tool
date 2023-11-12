//! Error handler for the tool.

use std::fmt;

#[derive(Debug)]
pub enum CLIError {
    IdentityError,
    IPResolutionError,
    ResourceBusy,
    WriterError,
}

impl fmt::Display for CLIError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CLIError::IdentityError => write!(f, "{}", IDENTITY_ERROR),
            CLIError::IPResolutionError => write!(f, "{}", IP_RESOLUTION_ERROR),
            CLIError::ResourceBusy => write!(f, "{}", RESOURCE_BUSY),
            CLIError::WriterError => write!(f, "{}", WRITER_FLUSH_ERROR),
        }
    }
}

const IDENTITY_ERROR: &str = "Identity failed to parse the string to get a peer MultiAddress. Please check the input address format.";
const IP_RESOLUTION_ERROR: &str = "Could not resolve IP from libp2p multiaddress.";
const RESOURCE_BUSY: &str = " System resources busy. Please restart the client.";
const WRITER_FLUSH_ERROR: &str = "Error occured while flusing buffer to file. Please check if your path is correct or if it has write access";
