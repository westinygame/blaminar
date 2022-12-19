use std::{io, net::SocketAddr};
use bytes::Bytes;
use crate::simulation::Message;
use bevy::prelude::Resource;

/// Events which can be received from the network.
#[derive(Debug, Resource)]
pub enum NetworkSimulationEvent {
    // A message was received from a remote client
    Message(SocketAddr, Bytes),
    // A new host has connected to us
    Connect(SocketAddr),
    // A host has disconnected from us
    Disconnect(SocketAddr),
    // An error occurred while receiving a message.
    RecvError(io::Error),
    // An error occurred while sending a message.
    SendError(io::Error, Message),
    // An error occurred while managing connections.
    ConnectionError(io::Error, Option<SocketAddr>),
}
