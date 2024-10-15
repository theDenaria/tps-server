use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use bevy::prelude::Resource;
use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};

use super::connection::{ConnectionConfig, NetworkInfo, UnityClient};
use super::error::{ClientNotFound, DisconnectReason};
use super::packet::Payload;
use super::transport::transport::{FromDenariaServerMessage, ToDenariaServerMessage};

/// Connection and disconnection events in the server.
#[derive(Debug, PartialEq, Eq)]
pub enum ServerEvent {
    ClientConnected {
        client_id: ClientId,
    },
    ClientDisconnected {
        client_id: ClientId,
        player_id: String,
        reason: DisconnectReason,
    },
}

#[derive(Debug, Resource)]
pub struct DenariaServer {
    connections: HashMap<ClientId, UnityClient>,
    player_connection_map: HashMap<String, ClientId>,
    connection_config: ConnectionConfig,
    events: VecDeque<ServerEvent>,
    from_transport_server_rx: Receiver<ToDenariaServerMessage>,
    to_transport_server_tx: Sender<FromDenariaServerMessage>,
}

impl DenariaServer {
    pub fn new(
        connection_config: ConnectionConfig,
        from_transport_server_rx: Receiver<ToDenariaServerMessage>,
        to_transport_server_tx: Sender<FromDenariaServerMessage>,
    ) -> Self {
        Self {
            connections: HashMap::new(),
            player_connection_map: HashMap::new(),
            connection_config,
            events: VecDeque::new(),
            from_transport_server_rx,
            to_transport_server_tx,
        }
    }

    /// Adds a new connection to the server. If a connection already exits it does nothing.
    /// <p style="background:rgba(77,220,255,0.16);padding:0.5em;">
    /// <strong>Note:</strong> This should only be called by the transport layer.
    /// </p>
    pub fn add_connection(&mut self, client_id: ClientId, player_id: String) {
        if self.connections.contains_key(&client_id) {
            return;
        }

        let mut connection = UnityClient::new_from_server(self.connection_config.clone());
        // Consider newly added connections as connected
        connection.set_connected(player_id.clone());
        self.connections.insert(client_id, connection);
        self.player_connection_map
            .insert(player_id.clone(), client_id);
        self.events
            .push_back(ServerEvent::ClientConnected { client_id })
    }

    pub fn get_event(&mut self) -> Option<ServerEvent> {
        self.events.pop_front()
    }

    /// Returns whether or not the server has connections
    pub fn has_connections(&self) -> bool {
        !self.connections.is_empty()
    }

    /// Returns the disconnection reason for the client if its disconnected
    pub fn disconnect_reason(&self, client_id: ClientId) -> Option<DisconnectReason> {
        if let Some(connection) = self.connections.get(&client_id) {
            return connection.disconnect_reason();
        }

        None
    }

    /// Returns the round-time trip for the client or 0.0 if the client is not found
    pub fn rtt(&self, client_id: ClientId) -> f64 {
        match self.connections.get(&client_id) {
            Some(connection) => connection.rtt(),
            None => 0.0,
        }
    }

    /// Returns the packet loss for the client or 0.0 if the client is not found
    pub fn packet_loss(&self, client_id: ClientId) -> f64 {
        match self.connections.get(&client_id) {
            Some(connection) => connection.packet_loss(),
            None => 0.0,
        }
    }

    /// Returns the bytes sent per seconds for the client or 0.0 if the client is not found
    pub fn bytes_sent_per_sec(&self, client_id: ClientId) -> f64 {
        match self.connections.get(&client_id) {
            Some(connection) => connection.bytes_sent_per_sec(),
            None => 0.0,
        }
    }

    /// Returns the bytes received per seconds for the client or 0.0 if the client is not found
    pub fn bytes_received_per_sec(&self, client_id: ClientId) -> f64 {
        match self.connections.get(&client_id) {
            Some(connection) => connection.bytes_received_per_sec(),
            None => 0.0,
        }
    }

    /// Returns all network informations for the client
    pub fn network_info(&self, client_id: ClientId) -> Result<NetworkInfo, ClientNotFound> {
        match self.connections.get(&client_id) {
            Some(connection) => Ok(connection.network_info()),
            None => Err(ClientNotFound),
        }
    }

    pub fn player_id(&self, client_id: ClientId) -> Result<&String, ClientNotFound> {
        match self.connections.get(&client_id) {
            Some(connection) => Ok(connection.player_id()),
            None => Err(ClientNotFound),
        }
    }

    pub fn client_id_by_player_id(&self, player_id: String) -> Result<ClientId, ClientNotFound> {
        match self.player_connection_map.get(&player_id) {
            Some(client_id) => Ok(*client_id),
            None => Err(ClientNotFound),
        }
    }

    /// Removes a connection from the server, emits an disconnect server event.
    /// It does nothing if the client does not exits.
    /// <p style="background:rgba(77,220,255,0.16);padding:0.5em;">
    /// <strong>Note:</strong> This should only be called by the transport layer.
    /// </p>
    pub fn remove_connection(&mut self, client_id: ClientId) {
        if let Some(connection) = self.connections.remove(&client_id) {
            let player_id = connection.player_id().clone();
            let reason = connection
                .disconnect_reason()
                .unwrap_or(DisconnectReason::Transport);
            self.events.push_back(ServerEvent::ClientDisconnected {
                client_id,
                player_id,
                reason,
            });
        }
    }

    /// Disconnects a client, it does nothing if the client does not exist.
    pub fn disconnect(&mut self, client_id: ClientId) {
        if let Some(connection) = self.connections.get_mut(&client_id) {
            connection.disconnect_with_reason(DisconnectReason::DisconnectedByServer)
        }
    }

    /// Disconnects all client.
    pub fn disconnect_all(&mut self) {
        for connection in self.connections.values_mut() {
            connection.disconnect_with_reason(DisconnectReason::DisconnectedByServer)
        }
    }

    /// Send a message to all clients over a channel.
    pub fn broadcast_message<I: Into<u8>, B: Into<Bytes>>(&mut self, channel_id: I, message: B) {
        let channel_id = channel_id.into();
        let message = message.into();
        for connection in self.connections.values_mut() {
            connection.send_message(channel_id, message.clone());
        }
    }

    /// Send a message to all clients, except the specified one, over a channel.
    pub fn broadcast_message_except<I: Into<u8>, B: Into<Bytes>>(
        &mut self,
        except_id: ClientId,
        channel_id: I,
        message: B,
    ) {
        let channel_id = channel_id.into();
        let message = message.into();
        for (connection_id, connection) in self.connections.iter_mut() {
            if except_id == *connection_id {
                continue;
            }

            connection.send_message(channel_id, message.clone());
        }
    }

    /// Returns the available memory in bytes of a channel for the given client.
    /// Returns 0 if the client is not found.
    pub fn channel_available_memory<I: Into<u8>>(
        &self,
        client_id: ClientId,
        channel_id: I,
    ) -> usize {
        match self.connections.get(&client_id) {
            Some(connection) => connection.channel_available_memory(channel_id),
            None => 0,
        }
    }

    /// Checks if can send a message with the given size in bytes over a channel for the given client.
    /// Returns false if the client is not found.
    pub fn can_send_message<I: Into<u8>>(
        &self,
        client_id: ClientId,
        channel_id: I,
        size_bytes: usize,
    ) -> bool {
        match self.connections.get(&client_id) {
            Some(connection) => connection.can_send_message(channel_id, size_bytes),
            None => false,
        }
    }

    /// Send a message to a client over a channel.
    pub fn send_message<I: Into<u8>, B: Into<Bytes>>(
        &mut self,
        client_id: ClientId,
        channel_id: I,
        message: B,
    ) {
        match self.connections.get_mut(&client_id) {
            Some(connection) => connection.send_message(channel_id, message),
            None => tracing::error!("Tried to send a message to invalid client {:?}", client_id),
        }
    }

    /// Receive a message from a client over a channel.
    pub fn receive_message<I: Into<u8>>(
        &mut self,
        client_id: ClientId,
        channel_id: I,
    ) -> Option<(Bytes, &String)> {
        if let Some(connection) = self.connections.get_mut(&client_id) {
            if let Some(message) = connection.receive_message(channel_id) {
                return Some((message, connection.player_id()));
            }
        }
        None
    }

    /// Return ids for all connected clients (iterator)
    pub fn clients_id_iter(&self) -> impl Iterator<Item = ClientId> + '_ {
        self.connections
            .iter()
            .filter(|(_, c)| c.is_connected())
            .map(|(id, _)| *id)
    }

    /// Return ids for all connected clients
    pub fn clients_id(&self) -> Vec<ClientId> {
        self.clients_id_iter().collect()
    }

    /// Return ids for all disconnected clients (iterator)
    pub fn disconnections_id_iter(&self) -> impl Iterator<Item = ClientId> + '_ {
        self.connections
            .iter()
            .filter(|(_, c)| c.is_disconnected())
            .map(|(id, _)| *id)
    }

    /// Return ids for all disconnected clients
    pub fn disconnections_id(&self) -> Vec<ClientId> {
        self.disconnections_id_iter().collect()
    }

    /// Returns the current number of connected clients.
    pub fn connected_clients(&self) -> usize {
        self.connections
            .iter()
            .filter(|(_, c)| c.is_connected())
            .count()
    }

    pub fn is_connected(&self, client_id: ClientId) -> bool {
        if let Some(connection) = self.connections.get(&client_id) {
            return connection.is_connected();
        }

        false
    }

    /// Advances the server by the duration.
    /// Should be called every tick
    pub fn update(&mut self, duration: Duration) {
        for connection in self.connections.values_mut() {
            connection.update(duration);
        }
    }

    /// Returns a list of packets to be sent to the client.
    /// <p style="background:rgba(77,220,255,0.16);padding:0.5em;">
    /// <strong>Note:</strong> This should only be called by the transport layer.
    /// </p>
    pub fn get_packets_to_send(
        &mut self,
        client_id: ClientId,
    ) -> Result<Vec<Payload>, ClientNotFound> {
        match self.connections.get_mut(&client_id) {
            Some(connection) => Ok(connection.get_packets_to_send()),
            None => Err(ClientNotFound),
        }
    }

    /// Process a packet received from the client.
    /// <p style="background:rgba(77,220,255,0.16);padding:0.5em;">
    /// <strong>Note:</strong> This should only be called by the transport layer.
    /// </p>
    pub fn process_packet_from(
        &mut self,
        payload: &[u8],
        client_id: ClientId,
    ) -> Result<(), ClientNotFound> {
        match self.connections.get_mut(&client_id) {
            Some(connection) => {
                connection.process_packet(payload);
                Ok(())
            }
            None => Err(ClientNotFound),
        }
    }

    pub fn process_server_transport_messages(&mut self) {
        while let Ok(message) = self.from_transport_server_rx.try_recv() {
            match message {
                ToDenariaServerMessage::ClientConnected {
                    client_id,
                    addr: _,
                    payload: _,
                    player_id,
                } => {
                    self.add_connection(ClientId::from_raw(client_id), player_id);
                }
                ToDenariaServerMessage::ClientDisconnected { client_id } => {
                    self.remove_connection(ClientId::from_raw(client_id));
                }
                ToDenariaServerMessage::Payload { client_id, payload } => {
                    tracing::debug!(
                        "Received payload from client: {:?}: payload: {:?}",
                        client_id,
                        payload
                    );
                    if let Err(e) =
                        self.process_packet_from(payload.as_slice(), ClientId::from_raw(client_id))
                    {
                        tracing::error!("Failed to process packet from client: {:?}", e);
                    }
                }
            }
        }
    }

    pub fn send_packets_to_server_transport(&mut self, client_id: ClientId, packets: Vec<Vec<u8>>) {
        if let Err(e) = self
            .to_transport_server_tx
            .send(FromDenariaServerMessage::SendPacket {
                client_id: client_id.raw(),
                packets,
            })
        {
            tracing::error!("Failed to send packet to server transport: {:?}", e);
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct ClientId(u64);

impl ClientId {
    /// Creates a [`ClientId`] from a raw 64 bit value.
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw 64 bit value of the [`ClientId`]
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl serde::Serialize for ClientId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ClientId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(ClientId::from_raw)
    }
}
