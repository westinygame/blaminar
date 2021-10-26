//! Network systems implementation backed by the Laminar network protocol.

use std::time::Instant;

use bytes::Bytes;
pub use laminar::{Config as LaminarConfig, ErrorKind, Socket as LaminarSocket, Packet, SocketEvent};
use bevy::log::{info, error};

use crate::simulation::{
    events::NetworkSimulationEvent,
    requirements::DeliveryRequirement,
    timing::{NetworkSimulationTime, network_simulation_time_system},
    transport::TransportResource,
};
use bevy::prelude::{Plugin, Res, ResMut, EventWriter, IntoSystem};
use bevy::app::AppBuilder;
use std::net::SocketAddr;

/// Use this plugin to add the laminar transport layer to your game.
pub struct LaminarPlugin {
    address:   SocketAddr,
    config: LaminarConfig
}

impl LaminarPlugin {
    pub fn new(address: SocketAddr, config: LaminarConfig) -> Self {
        LaminarPlugin { address, config }
    }
}

impl Plugin for LaminarPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_startup_system(log_startup.system())
            .add_event::<NetworkSimulationEvent>()
            .init_resource::<NetworkSimulationTime>()
            .init_resource::<TransportResource>()
            .insert_resource(LaminarSocketResource::new(
                LaminarSocket::bind_with_config(self.address, self.config.clone()).ok()))
            .add_system(network_simulation_time_system.system())
            .add_system(laminar_network_send_system.system())
            .add_system(laminar_network_poll_system.system())
            .add_system(laminar_network_recv_system.system());
    }

    fn name(&self) -> &str {
        "laminar"
    }
}

fn log_startup(socket: Res<LaminarSocketResource>) {
    info!("Start listening on {}", socket.get().unwrap().local_addr().unwrap());
}

/// Creates a new laminar network send system.
pub fn laminar_network_send_system(mut transport: ResMut<TransportResource>,
                               mut socket:        ResMut<LaminarSocketResource>,
                               mut event_channel: EventWriter<NetworkSimulationEvent>,
                                   sim_time:      Res<NetworkSimulationTime>) {

    if let Some(socket) = socket.get_mut() {
        let messages = transport
            .drain_messages_to_send(|_| sim_time.should_send_message_now());

        for message in messages {
            let packet = match message.delivery {
                DeliveryRequirement::Unreliable => {
                    Packet::unreliable(
                        message.destination,
                        message.payload.to_vec(),
                    )
                }
                DeliveryRequirement::UnreliableSequenced(stream_id) => {
                    Packet::unreliable_sequenced(
                        message.destination,
                        message.payload.to_vec(),
                        stream_id,
                    )
                }
                DeliveryRequirement::Reliable => {
                    Packet::reliable_unordered(
                        message.destination,
                        message.payload.to_vec(),
                    )
                }
                DeliveryRequirement::ReliableSequenced(stream_id) => {
                    Packet::reliable_sequenced(
                        message.destination,
                        message.payload.to_vec(),
                        stream_id,
                    )
                }
                DeliveryRequirement::ReliableOrdered(stream_id) => {
                    Packet::reliable_ordered(
                        message.destination,
                        message.payload.to_vec(),
                        stream_id,
                    )
                }
                DeliveryRequirement::Default => {
                    Packet::reliable_ordered(
                        message.destination,
                        message.payload.to_vec(),
                        None,
                    )
                }
            };

            match socket.send(packet) {
                Err(ErrorKind::IOError(e)) => {
                    event_channel.send(
                        NetworkSimulationEvent::SendError(e, message),
                    );
                }
                Err(e) => {
                    error!("Error sending message: {:?}", e);
                }
                Ok(_) => {}
            }
        }
    }
}

/// Creates a new laminar network poll system.
pub fn laminar_network_poll_system(mut socket: ResMut<LaminarSocketResource>) {
    if let Some(socket) = socket.get_mut() {
        socket.manual_poll(Instant::now());
    }
}

/// Creates a new laminar receive system.
pub fn laminar_network_recv_system(mut socket:        ResMut<LaminarSocketResource>,
                                   mut event_channel: EventWriter<NetworkSimulationEvent>) {
    if let Some(socket) = socket.get_mut() {
        while let Some(event) = socket.recv() {
            let event = match event {
                SocketEvent::Packet(packet) => {
                    NetworkSimulationEvent::Message(
                        packet.addr(),
                        Bytes::copy_from_slice(packet.payload()),
                    )
                }
                SocketEvent::Disconnect(addr) | SocketEvent::Timeout(addr) => {
                    NetworkSimulationEvent::Disconnect(addr)
                }
                SocketEvent::Connect(addr) => NetworkSimulationEvent::Connect(addr),
            };
            event_channel.send(event);
        }
    }
}

/// Resource that owns the Laminar socket.
pub struct LaminarSocketResource {
    socket: Option<LaminarSocket>,
}

impl Default for LaminarSocketResource {
    fn default() -> Self {
        Self { socket: None }
    }
}

impl LaminarSocketResource {
    /// Creates a new instance of the `UdpSocketResource`.
    #[must_use]
    pub fn new(socket: Option<LaminarSocket>) -> Self {
        Self { socket }
    }

    /// Returns a reference to the socket if there is one configured.
    #[must_use]
    pub fn get(&self) -> Option<&LaminarSocket> {
        self.socket.as_ref()
    }

    /// Returns a mutable reference to the socket if there is one configured.
    pub fn get_mut(&mut self) -> Option<&mut LaminarSocket> {
        self.socket.as_mut()
    }

    /// Sets the bound socket to the `LaminarSocketResource`.
    pub fn set_socket(&mut self, socket: LaminarSocket) {
        self.socket = Some(socket);
    }

    /// Drops the socket from the `LaminarSocketResource`.
    pub fn drop_socket(&mut self) {
        self.socket = None;
    }
}
