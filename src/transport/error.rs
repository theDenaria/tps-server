use crate::error::DisconnectReason;
use std::{error::Error, fmt};

use super::server::error::TransportServerError;

#[derive(Debug)]
pub enum TransportError {
    Server(TransportServerError),
    Matta(DisconnectReason),
    IO(std::io::Error),
}

impl Error for TransportError {}

impl fmt::Display for TransportError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TransportError::Server(ref err) => err.fmt(fmt),
            TransportError::Matta(ref err) => err.fmt(fmt),
            TransportError::IO(ref err) => err.fmt(fmt),
        }
    }
}

impl From<TransportServerError> for TransportError {
    fn from(inner: TransportServerError) -> Self {
        TransportError::Server(inner)
    }
}

impl From<DisconnectReason> for TransportError {
    fn from(inner: DisconnectReason) -> Self {
        TransportError::Matta(inner)
    }
}

impl From<std::io::Error> for TransportError {
    fn from(inner: std::io::Error) -> Self {
        TransportError::IO(inner)
    }
}
