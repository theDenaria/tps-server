use std::{
    io,
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

use bevy_ecs::system::Resource;

use crate::{
    constants::TRANSPORT_MAX_PACKET_BYTES,
    server::{ClientId, MattaServer},
};

use super::{
    error::TransportError,
    server::server::{ServerConfig, ServerResult, TransportServer},
};
#[derive(Debug, Resource)]
pub struct ServerTransport {
    socket: UdpSocket,
    transport_server: TransportServer,
    buffer: [u8; TRANSPORT_MAX_PACKET_BYTES],
}

impl ServerTransport {
    pub fn new(server_config: ServerConfig, socket: UdpSocket) -> Result<Self, std::io::Error> {
        socket.set_nonblocking(true)?;

        let transport_server = TransportServer::new(server_config);

        Ok(Self {
            socket,
            transport_server,
            buffer: [0; TRANSPORT_MAX_PACKET_BYTES],
        })
    }

    /// Returns the server public address
    pub fn addresses(&self) -> Vec<SocketAddr> {
        self.transport_server.addresses()
    }

    /// Returns the maximum number of clients that can be connected.
    pub fn max_clients(&self) -> usize {
        self.transport_server.max_clients()
    }

    /// Returns current number of clients connected.
    pub fn connected_clients(&self) -> usize {
        self.transport_server.connected_clients()
    }

    /// Returns the client address if connected.
    pub fn client_addr(&self, client_id: ClientId) -> Option<SocketAddr> {
        self.transport_server.client_addr(client_id.raw())
    }

    /// Disconnects all connected clients.
    /// This sends the disconnect packet instantly, use this when closing/exiting games,
    pub fn disconnect_all(&mut self, server: &mut MattaServer) {
        for client_id in self.transport_server.clients_id() {
            let server_result = self.transport_server.disconnect(client_id);
            handle_server_result(server_result, &self.socket, server);
        }
    }

    /// Returns the duration since the connected client last received a packet.
    /// Usefull to detect users that are timing out.
    pub fn time_since_last_received_packet(&self, client_id: ClientId) -> Option<Duration> {
        self.transport_server
            .time_since_last_received_packet(client_id.raw())
    }

    /// Advances the transport by the duration, and receive packets from the network.
    pub fn update(
        &mut self,
        duration: Duration,
        server: &mut MattaServer,
    ) -> Result<(), TransportError> {
        self.transport_server.update(duration);

        loop {
            match self.socket.recv_from(&mut self.buffer) {
                Ok((len, addr)) => {
                    let server_result = self
                        .transport_server
                        .process_packet(addr, &mut self.buffer[..len]);
                    handle_server_result(server_result, &self.socket, server);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(ref e) if e.kind() == io::ErrorKind::ConnectionReset => continue,
                Err(e) => return Err(e.into()),
            };
        }

        for client_id in self.transport_server.clients_id() {
            let server_result = self.transport_server.update_client(client_id);
            handle_server_result(server_result, &self.socket, server);
        }

        for disconnection_id in server.disconnections_id() {
            let server_result = self.transport_server.disconnect(disconnection_id.raw());
            handle_server_result(server_result, &self.socket, server);
        }

        Ok(())
    }

    /// Send packets to connected clients.
    pub fn send_packets(&mut self, server: &mut MattaServer) {
        'clients: for client_id in server.clients_id() {
            let packets = server.get_packets_to_send(client_id).unwrap();
            for packet in packets {
                match self
                    .transport_server
                    .generate_payload_packet(client_id.raw(), &packet)
                {
                    Ok((addr, payload)) => {
                        if let Err(e) = self.socket.send_to(payload, addr) {
                            tracing::error!(
                                "Failed to send packet to client {client_id} ({addr}): {e}"
                            );
                            continue 'clients;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to encrypt payload packet for client {client_id}: {e}"
                        );
                        continue 'clients;
                    }
                }
            }
        }
    }
}

fn handle_server_result(
    server_result: ServerResult,
    socket: &UdpSocket,
    reliable_server: &mut MattaServer,
) {
    let send_packet = |packet: &[u8], addr: SocketAddr| {
        if let Err(err) = socket.send_to(packet, addr) {
            tracing::error!("Failed to send packet to {addr}: {err}");
        }
    };

    match server_result {
        ServerResult::None => {}
        ServerResult::PacketToSend { payload, addr } => {
            send_packet(payload, addr);
        }
        ServerResult::Payload { client_id, payload } => {
            let client_id = ClientId::from_raw(client_id);
            if let Err(e) = reliable_server.process_packet_from(payload, client_id) {
                tracing::error!("Error while processing payload for {}: {}", client_id, e);
            }
        }
        ServerResult::ClientConnected {
            client_id,
            addr,
            payload,
            player_id,
        } => {
            reliable_server.add_connection(ClientId::from_raw(client_id), player_id);
            send_packet(payload, addr);
        }
        ServerResult::ClientDisconnected {
            client_id,
            addr,
            payload,
        } => {
            reliable_server.remove_connection(ClientId::from_raw(client_id));
            if let Some(payload) = payload {
                send_packet(payload, addr);
            }
        }
    }
}
