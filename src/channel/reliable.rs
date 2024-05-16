use std::{
    collections::{btree_map, BTreeMap},
    time::Duration,
};

use bytes::Bytes;

use crate::{constants::MAX_MESSAGES_LENGTH, error::ChannelError, packet::Packet};

#[derive(Debug)]
enum UnackedMessage {
    Small {
        message: Bytes,
        last_sent: Option<Duration>,
    },
}

#[derive(Debug)]
pub struct SendChannelReliable {
    channel_id: u8,
    unacked_messages: BTreeMap<u64, UnackedMessage>,
    next_package_sequence_id: u16,
    next_message_id: u64,
    resend_time: Duration,
    max_memory_usage_bytes: usize,
    memory_usage_bytes: usize,
}

#[derive(Debug)]
enum ReliableOrder {
    Ordered,
}

#[derive(Debug)]
pub struct ReceiveChannelReliable {
    messages: BTreeMap<u64, Bytes>,
    oldest_pending_message_id: u64,
    reliable_order: ReliableOrder,
    memory_usage_bytes: usize,
    max_memory_usage_bytes: usize,
}

impl SendChannelReliable {
    pub fn new(channel_id: u8, resend_time: Duration, max_memory_usage_bytes: usize) -> Self {
        Self {
            channel_id,
            unacked_messages: BTreeMap::new(),
            next_package_sequence_id: 0,
            next_message_id: 0,
            resend_time,
            max_memory_usage_bytes,
            memory_usage_bytes: 0,
        }
    }

    pub fn available_memory(&self) -> usize {
        self.max_memory_usage_bytes - self.memory_usage_bytes
    }

    pub fn can_send_message(&self, size_bytes: usize) -> bool {
        size_bytes + self.memory_usage_bytes <= self.max_memory_usage_bytes
    }

    pub fn get_packets_to_send(
        &mut self,
        available_bytes: &mut u64,
        current_time: Duration,
    ) -> Vec<Packet> {
        if self.unacked_messages.is_empty() {
            return vec![];
        }

        let mut packets: Vec<Packet> = vec![];

        let mut small_messages: Vec<(u64, Bytes)> = vec![];
        let mut small_messages_bytes = 0;

        for (&message_id, unacked_message) in self.unacked_messages.iter_mut() {
            match unacked_message {
                UnackedMessage::Small { message, last_sent } => {
                    if *available_bytes < message.len() as u64 {
                        // Skip message, no bytes available to send this message
                        continue;
                    }

                    if let Some(last_sent) = last_sent {
                        if current_time - *last_sent < self.resend_time {
                            continue;
                        }
                    }

                    *available_bytes -= message.len() as u64;

                    // Generate packet with small messages if you cannot fit
                    let serialized_size = message.len()
                        + octets::varint_len(message.len() as u64)
                        + octets::varint_len(message_id);
                    if small_messages_bytes + serialized_size > MAX_MESSAGES_LENGTH {
                        packets.push(Packet::SmallReliable {
                            channel_id: self.channel_id,
                            packet_type: 0,
                            packet_process_time: 0,
                            sequence_id: self.next_package_sequence_id,
                            acked_seq_id: u16::MAX,
                            acked_mask: 0,
                            messages: std::mem::take(&mut small_messages),
                        });
                        small_messages_bytes = 0;
                        self.next_package_sequence_id += 1;
                    }

                    small_messages_bytes += serialized_size;
                    small_messages.push((message_id, message.clone()));
                    *last_sent = Some(current_time);

                    continue;
                }
            }
        }

        // Generate final packet for remaining small messages
        if !small_messages.is_empty() {
            packets.push(Packet::SmallReliable {
                channel_id: self.channel_id,
                packet_type: 0,
                packet_process_time: 0,
                sequence_id: self.next_package_sequence_id,
                acked_seq_id: u16::MAX,
                acked_mask: 0,
                messages: std::mem::take(&mut small_messages),
            });
            self.next_package_sequence_id += 1;
        }
        packets
    }

    pub fn send_message(&mut self, message: Bytes) -> Result<(), ChannelError> {
        if self.memory_usage_bytes + message.len() > self.max_memory_usage_bytes {
            return Err(ChannelError::ReliableChannelMaxMemoryReached);
        }

        self.memory_usage_bytes += message.len();
        let unacked_message = UnackedMessage::Small {
            message,
            last_sent: None,
        };

        self.unacked_messages
            .insert(self.next_message_id, unacked_message);
        self.next_message_id += 1;

        Ok(())
    }

    pub fn process_message_ack(&mut self, message_id: u64) {
        if self.unacked_messages.contains_key(&message_id) {
            tracing::trace!("MESSAGE ID: {:?} IS ACKEDD!!!", message_id);
            let unacked_message = self.unacked_messages.remove(&message_id).unwrap();
            let UnackedMessage::Small {
                message: payload, ..
            } = unacked_message;

            self.memory_usage_bytes -= payload.len();
        }
    }
}

impl ReceiveChannelReliable {
    pub fn new(max_memory_usage_bytes: usize, ordered: Option<bool>) -> Self {
        let ordered = ordered.unwrap_or(true);
        let reliable_order = match ordered {
            true => ReliableOrder::Ordered,
            false => {
                unreachable!("Reliable unordered is not supported yet");
            }
        };
        Self {
            messages: BTreeMap::new(),
            oldest_pending_message_id: 0,
            reliable_order,
            memory_usage_bytes: 0,
            max_memory_usage_bytes,
        }
    }

    pub fn process_message(&mut self, message: Bytes, message_id: u64) -> Result<(), ChannelError> {
        if message_id < self.oldest_pending_message_id {
            // Discard old message already received
            return Ok(());
        }

        match &mut self.reliable_order {
            ReliableOrder::Ordered => {
                if let btree_map::Entry::Vacant(entry) = self.messages.entry(message_id) {
                    if self.memory_usage_bytes + message.len() > self.max_memory_usage_bytes {
                        return Err(ChannelError::ReliableChannelMaxMemoryReached);
                    }
                    self.memory_usage_bytes += message.len();

                    entry.insert(message);
                }
            }
        }

        Ok(())
    }

    pub fn receive_message(&mut self) -> Option<Bytes> {
        match &mut self.reliable_order {
            ReliableOrder::Ordered => {
                let Some(message) = self.messages.remove(&self.oldest_pending_message_id) else {
                    return None;
                };

                self.oldest_pending_message_id += 1;
                self.memory_usage_bytes -= message.len();
                Some(message)
            }
        }
    }
}
