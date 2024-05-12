use crate::channel::reliable::{ReceiveChannelReliable, SendChannelReliable};
use crate::channel::unreliable::{ReceiveChannelUnreliable, SendChannelUnreliable};
use crate::channel::{ChannelConfig, DefaultChannel, SendType};
use crate::connection_stats::ConnectionStats;
use crate::error::DisconnectReason;
use crate::packet::{Packet, Payload};
use bytes::Bytes;
use octets::OctetsMut;

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// The number of bytes that is available per update tick to send messages.
    /// Default: 60_000, at 60hz this is becomes 28.8 Mbps
    pub available_bytes_per_tick: u64,
    /// The channels that the server sends to the client.
    /// The order of the channels in this Vec determines which channel has priority when generating packets.
    /// Each tick, the first channel can consume up to `available_bytes_per_tick`,
    /// used bytes are removed from it and passed to the next channel
    pub server_channels_config: Vec<ChannelConfig>,
    /// The channels that the client sends to the server.
    /// The order of the channels in this Vec determines which channel has priority when generating packets.
    /// Each tick, the first channel can consume up to `available_bytes_per_tick`,
    /// used bytes are removed from it and passed to the next channel
    pub client_channels_config: Vec<ChannelConfig>,
}

#[derive(Debug, Clone)]
struct PacketSent {
    sent_at: Duration,
    info: PacketSentInfo,
}

#[derive(Debug, Clone)]
enum PacketSentInfo {
    // No need to track info for unreliable messages
    None,
    ReliableMessages {
        channel_id: u8,
        message_ids: Vec<u64>,
    },
}

#[derive(Debug)]
enum ChannelOrder {
    Reliable(u8),
    Unreliable(u8),
}

/// Describes the stats of a connection.
pub struct NetworkInfo {
    /// Round-trip Time
    pub rtt: f64,
    pub packet_loss: f64,
    pub bytes_sent_per_second: f64,
    pub bytes_received_per_second: f64,
}

#[derive(Debug)]
pub enum ClientConnectionStatus {
    Connected,
    Connecting,
    Disconnected { reason: DisconnectReason },
}

#[derive(Debug)]
pub struct UnityClient {
    current_time: Duration,
    sent_packets: BTreeMap<u16, PacketSent>,
    pending_acks: VecDeque<u16>,
    new_ack_to_send: bool,
    ack_process_start_instant: Instant,
    channel_send_order: Vec<ChannelOrder>,
    send_unreliable_channel: SendChannelUnreliable,
    receive_unreliable_channel: ReceiveChannelUnreliable,
    send_reliable_channel: SendChannelReliable,
    receive_reliable_channel: ReceiveChannelReliable,
    stats: ConnectionStats,
    available_bytes_per_tick: u64,
    connection_status: ClientConnectionStatus,
    rtt: f64,
    player_id: String,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            // At 60hz this is becomes 28.8 Mbps
            available_bytes_per_tick: 60_000,
            server_channels_config: DefaultChannel::config(),
            client_channels_config: DefaultChannel::config(),
        }
    }
}

impl UnityClient {
    pub fn new(config: ConnectionConfig) -> Self {
        Self::from_channels(
            config.available_bytes_per_tick,
            config.server_channels_config[0].clone(),
            config.server_channels_config[1].clone(),
            config.client_channels_config[0].clone(),
            config.client_channels_config[1].clone(),
        )
    }

    // When creating a client from the server, the server_channels_config are used as send channels,
    // and the client_channels_config is used as recv channels.
    pub(crate) fn new_from_server(config: ConnectionConfig) -> Self {
        Self::from_channels(
            config.available_bytes_per_tick,
            config.server_channels_config[0].clone(),
            config.server_channels_config[1].clone(),
            config.client_channels_config[0].clone(),
            config.client_channels_config[1].clone(),
        )
    }

    fn from_channels(
        available_bytes_per_tick: u64,
        send_unreliable_channel_config: ChannelConfig,
        send_reliable_channel_config: ChannelConfig,
        receive_unreliable_channel_config: ChannelConfig,
        receive_reliable_channel_config: ChannelConfig,
    ) -> Self {
        let send_unreliable_channel = SendChannelUnreliable::new(
            send_unreliable_channel_config.channel_id,
            send_unreliable_channel_config.max_memory_usage_bytes,
        );

        let send_reliable_resend_time;
        match send_reliable_channel_config.send_type {
            SendType::ReliableOrdered { resend_time } => {
                send_reliable_resend_time = resend_time;
            }
            _ => {
                unreachable!("Shouldn't come here for send_reliable_channel_config.send_type")
            }
        }

        let send_reliable_channel = SendChannelReliable::new(
            send_reliable_channel_config.channel_id,
            send_reliable_resend_time,
            send_reliable_channel_config.max_memory_usage_bytes,
        );

        let mut channel_send_order: Vec<ChannelOrder> = Vec::with_capacity(2);

        channel_send_order.push(ChannelOrder::Unreliable(
            send_reliable_channel_config.channel_id,
        ));
        channel_send_order.push(ChannelOrder::Unreliable(
            send_unreliable_channel_config.channel_id,
        ));

        let receive_unreliable_channel = ReceiveChannelUnreliable::new(
            receive_unreliable_channel_config.channel_id,
            receive_unreliable_channel_config.max_memory_usage_bytes,
        );

        let receive_reliable_channel = ReceiveChannelReliable::new(
            receive_reliable_channel_config.max_memory_usage_bytes,
            Some(true),
        );

        Self {
            current_time: Duration::ZERO,
            sent_packets: BTreeMap::new(),
            pending_acks: VecDeque::with_capacity(32),
            new_ack_to_send: false,
            ack_process_start_instant: Instant::now(),
            channel_send_order,
            send_unreliable_channel,
            receive_unreliable_channel,
            send_reliable_channel,
            receive_reliable_channel,
            stats: ConnectionStats::new(),
            rtt: 0.0,
            available_bytes_per_tick,
            connection_status: ClientConnectionStatus::Connecting,
            player_id: String::new(),
        }
    }

    /// Returns the round-time trip for the connection.
    pub fn rtt(&self) -> f64 {
        self.rtt
    }

    /// Returns the packet loss for the connection.
    pub fn packet_loss(&self) -> f64 {
        self.stats.packet_loss()
    }

    /// Returns the bytes sent per second in the connection.
    pub fn bytes_sent_per_sec(&self) -> f64 {
        self.stats.bytes_sent_per_second(self.current_time)
    }

    /// Returns the bytes received per second in the connection.
    pub fn bytes_received_per_sec(&self) -> f64 {
        self.stats.bytes_received_per_second(self.current_time)
    }

    /// Returns all network informations for the connection.
    pub fn network_info(&self) -> NetworkInfo {
        NetworkInfo {
            rtt: self.rtt,
            packet_loss: self.stats.packet_loss(),
            bytes_sent_per_second: self.stats.bytes_sent_per_second(self.current_time),
            bytes_received_per_second: self.stats.bytes_received_per_second(self.current_time),
        }
    }

    /// Returns whether the client is connected.
    #[inline]
    pub fn is_connected(&self) -> bool {
        matches!(self.connection_status, ClientConnectionStatus::Connected)
    }

    /// Returns whether the client is connecting.
    #[inline]
    pub fn is_connecting(&self) -> bool {
        matches!(self.connection_status, ClientConnectionStatus::Connecting)
    }

    /// Returns whether the client is disconnected.
    #[inline]
    pub fn is_disconnected(&self) -> bool {
        matches!(
            self.connection_status,
            ClientConnectionStatus::Disconnected { .. }
        )
    }

    /// Returns the disconnect reason if the client is disconnected.
    pub fn disconnect_reason(&self) -> Option<DisconnectReason> {
        if let ClientConnectionStatus::Disconnected { reason } = self.connection_status {
            Some(reason)
        } else {
            None
        }
    }

    /// Set the client connection status to connected.
    ///
    /// Does nothing if the client is disconnected. A disconnected client must be reconstructed.
    ///
    /// <p style="background:rgba(77,220,255,0.16);padding:0.5em;">
    /// <strong>Note:</strong> This should only be called by the transport layer.
    /// </p>
    pub fn set_connected(&mut self, player_id: String) {
        if !self.is_disconnected() {
            self.connection_status = ClientConnectionStatus::Connected;
            self.player_id = player_id;
        }
    }

    pub fn player_id(&self) -> String {
        self.player_id.clone()
    }

    /// Set the client connection status to connecting.
    ///
    /// Does nothing if the client is disconnected. A disconnected client must be reconstructed.
    ///
    /// <p style="background:rgba(77,220,255,0.16);padding:0.5em;">
    /// <strong>Note:</strong> This should only be called by the transport layer.
    /// </p>
    pub fn set_connecting(&mut self) {
        if !self.is_disconnected() {
            self.connection_status = ClientConnectionStatus::Connecting;
        }
    }

    /// Disconnect the client.
    ///
    /// If the client is already disconnected, it does nothing.
    pub fn disconnect(&mut self) {
        self.disconnect_with_reason(DisconnectReason::DisconnectedByClient);
    }

    /// Disconnect the client because an error occurred in the transport layer.
    ///
    /// If the client is already disconnected, it does nothing.
    /// <p style="background:rgba(77,220,255,0.16);padding:0.5em;">
    /// <strong>Note:</strong> This should only be called by the transport layer.
    /// </p>
    pub fn disconnect_due_to_transport(&mut self) {
        self.disconnect_with_reason(DisconnectReason::Transport);
    }

    /// Returns the available memory in bytes for the given channel.
    pub fn channel_available_memory<I: Into<u8>>(&self, channel_id: I) -> usize {
        let channel_id = channel_id.into();
        match channel_id {
            0 => self.send_unreliable_channel.available_memory(),
            1 => self.send_reliable_channel.available_memory(),
            _ => panic!("Called 'channel_available_memory' with invalid channel {channel_id}"),
        }
    }

    /// Checks if the channel can send a message with the given size in bytes.
    pub fn can_send_message<I: Into<u8>>(&self, channel_id: I, size_bytes: usize) -> bool {
        let channel_id = channel_id.into();
        match channel_id {
            0 => self.send_unreliable_channel.can_send_message(size_bytes),
            1 => self.send_reliable_channel.can_send_message(size_bytes),
            _ => panic!("Called 'can_send_message' with invalid channel {channel_id}"),
        }
    }

    /// Send a message to the server over a channel.
    pub fn send_message<I: Into<u8>, B: Into<Bytes>>(&mut self, channel_id: I, message: B) {
        if self.is_disconnected() {
            return;
        }

        let channel_id = channel_id.into();
        match channel_id {
            0 => {
                self.send_unreliable_channel.send_message(message.into());
            }
            1 => {
                if let Err(error) = self.send_reliable_channel.send_message(message.into()) {
                    self.disconnect_with_reason(DisconnectReason::SendChannelError {
                        channel_id,
                        error,
                    });
                }
            }
            _ => {
                panic!("Called 'send_message' with invalid channel {channel_id}");
            }
        }
    }

    /// Receive a message from the server over a channel.
    pub fn receive_message<I: Into<u8>>(&mut self, channel_id: I) -> Option<Bytes> {
        if self.is_disconnected() {
            return None;
        }

        let channel_id = channel_id.into();

        let channel_id = channel_id.into();
        match channel_id {
            0 => self.receive_unreliable_channel.receive_message(),
            1 => self.receive_reliable_channel.receive_message(),
            _ => panic!("Called 'receive_message' with invalid channel {channel_id}"),
        }
    }

    /// Advances the client by the duration.
    /// Should be called every tick
    pub fn update(&mut self, duration: Duration) {
        self.current_time += duration;
        self.stats.update(self.current_time);

        // Discard lost packets
        let mut lost_packets: Vec<u16> = Vec::new();
        for (&sequence, sent_packet) in self.sent_packets.iter() {
            const DISCARD_AFTER: Duration = Duration::from_secs(3);
            if self.current_time - sent_packet.sent_at >= DISCARD_AFTER {
                lost_packets.push(sequence);
            } else {
                // If the current packet is not lost, the next ones will not be lost
                // since all the next packets were sent after this one.
                break;
            }
        }

        for sequence in lost_packets.iter() {
            self.sent_packets.remove(sequence);
        }
    }

    /// Process a packet received from the server.
    /// <p style="background:rgba(77,220,255,0.16);padding:0.5em;">
    /// <strong>Note:</strong> This should only be called by the transport layer.
    /// </p>
    pub fn process_packet(&mut self, packet: &[u8]) {
        if self.is_disconnected() {
            return;
        }

        self.stats.received_packet(packet.len() as u64);
        let mut octets = octets::Octets::with_slice(packet);
        let packet = match Packet::from_bytes(&mut octets) {
            Err(err) => {
                self.disconnect_with_reason(DisconnectReason::PacketDeserialization(err));
                return;
            }
            Ok(packet) => packet,
        };

        match packet {
            Packet::SmallReliable {
                channel_id,
                messages,
                sequence_id,
                ..
            } => {
                self.add_pending_ack(sequence_id);
                for (message_id, message) in messages {
                    if let Err(error) = self
                        .receive_reliable_channel
                        .process_message(message, message_id)
                    {
                        self.disconnect_with_reason(DisconnectReason::ReceiveChannelError {
                            channel_id,
                            error,
                        });
                        return;
                    }
                }
            }
            Packet::SmallUnreliable { messages, .. } => {
                for message in messages {
                    self.receive_unreliable_channel.process_message(message);
                }
            }

            Packet::Ack {
                acked_seq_id,
                acked_mask,
                ..
            } => {
                // Create list with just new acks
                // This prevents DoS from huge ack ranges
                let new_acks = Self::get_acked_packet_ids(acked_seq_id, acked_mask);

                for packet_sequence in new_acks {
                    if let Some(sent_packet) = self.sent_packets.remove(&packet_sequence) {
                        self.stats
                            .acked_packet(sent_packet.sent_at, self.current_time);

                        // Update rtt
                        let rtt = (self.current_time - sent_packet.sent_at).as_secs_f64();
                        if self.rtt < f64::EPSILON {
                            self.rtt = rtt;
                        } else {
                            self.rtt = self.rtt * 0.875 + rtt * 0.125;
                        }

                        match sent_packet.info {
                            PacketSentInfo::ReliableMessages {
                                channel_id: _,
                                message_ids,
                            } => {
                                for message_id in message_ids {
                                    self.send_reliable_channel.process_message_ack(message_id);
                                }
                            }
                            PacketSentInfo::None => {}
                        }
                    }
                }
            }
        }
    }

    /// Returns a list of packets to be sent to the server.
    /// <p style="background:rgba(77,220,255,0.16);padding:0.5em;">
    /// <strong>Note:</strong> This should only be called by the transport layer.
    /// </p>
    pub fn get_packets_to_send(&mut self) -> Vec<Payload> {
        let mut packets: Vec<Packet> = vec![];
        if self.is_disconnected() {
            return vec![];
        }

        let mut available_bytes = self.available_bytes_per_tick;
        for order in self.channel_send_order.iter() {
            match order {
                ChannelOrder::Reliable(_channel_id) => {
                    packets.append(
                        &mut self
                            .send_reliable_channel
                            .get_packets_to_send(&mut available_bytes, self.current_time),
                    );
                }
                ChannelOrder::Unreliable(_channel_id) => {
                    packets.append(
                        &mut self
                            .send_unreliable_channel
                            .get_packets_to_send(&mut available_bytes),
                    );
                }
            }
        }

        if self.new_ack_to_send {
            if let Some((ack_seq_id, ack_mask)) = self.create_acked_bytes() {
                let ack_packet = Packet::Ack {
                    channel_id: 1,
                    packet_type: 1,
                    packet_process_time: self.ack_process_start_instant.elapsed().as_millis()
                        as u16,
                    sequence_id: 0,
                    acked_seq_id: ack_seq_id,
                    acked_mask: ack_mask,
                    end_posfix: 0,
                };
                packets.push(ack_packet);
            }
        }

        let sent_at = self.current_time;
        for packet in packets.iter() {
            match packet {
                Packet::SmallReliable {
                    sequence_id,
                    channel_id,
                    messages,
                    ..
                } => {
                    self.sent_packets.insert(
                        *sequence_id,
                        PacketSent {
                            sent_at,
                            info: PacketSentInfo::ReliableMessages {
                                channel_id: *channel_id,
                                message_ids: messages.iter().map(|(id, _)| *id).collect(),
                            },
                        },
                    );
                }
                _ => {}
            }
        }

        let mut buffer = [0u8; 1400];
        let mut serialized_packets = Vec::with_capacity(packets.len());
        let mut bytes_sent: u64 = 0;
        for packet in packets {
            let mut oct = OctetsMut::with_slice(&mut buffer);
            let len = match packet.to_bytes(&mut oct) {
                Err(err) => {
                    self.disconnect_with_reason(DisconnectReason::PacketSerialization(err));
                    return vec![];
                }
                Ok(len) => len,
            };

            bytes_sent += len as u64;
            serialized_packets.push(buffer[..len].to_vec());
        }

        self.stats
            .sent_packets(serialized_packets.len() as u64, bytes_sent);

        serialized_packets
    }

    fn add_pending_ack(&mut self, sequence_id: u16) {
        if self.pending_acks[0] >= sequence_id || self.pending_acks.contains(&sequence_id) {
            return;
        }
        self.new_ack_to_send = true;
        self.ack_process_start_instant = Instant::now();
        if self.pending_acks.len() >= 32 {
            self.pending_acks.pop_front();
        }
        let index_to_insert = self
            .pending_acks
            .iter()
            .position(|&x| x > sequence_id)
            .unwrap_or(self.pending_acks.len());
        self.pending_acks.insert(index_to_insert, sequence_id);
    }

    pub fn create_acked_bytes(&self) -> Option<(u16, u32)> {
        if let Some(last_pending_ack) = self.pending_acks.back() {
            let mut seq_id = last_pending_ack.clone();
            let mut ack_mask = 0u32;
            for i in 0..32 {
                if self.pending_acks.contains(&seq_id) {
                    ack_mask |= 1 << i; // Write 1 to ack_mask if sequence ID exists
                }
                seq_id = seq_id - 1; // Move to the next sequence ID
            }
            return Some((*last_pending_ack, ack_mask));
        }
        None
    }

    pub fn get_acked_packet_ids(ack_seq: u16, ack_mask: u32) -> Vec<u16> {
        let mut acked_seqs = vec![];
        // Process the ack_mask for additional acknowledgments
        for i in 0..32 {
            if ack_mask & (1 << i) != 0 {
                let seq = ack_seq.wrapping_sub(i as u16);
                acked_seqs.push(seq);
            }
        }
        acked_seqs
    }

    pub(crate) fn disconnect_with_reason(&mut self, reason: DisconnectReason) {
        if !self.is_disconnected() {
            self.connection_status = ClientConnectionStatus::Disconnected { reason };
        }
    }
}
