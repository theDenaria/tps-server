use std::collections::VecDeque;

use bytes::Bytes;

use crate::{constants::MAX_MESSAGES_LENGTH, packet::Packet};

#[derive(Debug)]
pub struct SendChannelUnreliable {
    channel_id: u8,
    unreliable_messages: VecDeque<Bytes>,
    max_memory_usage_bytes: usize,
    memory_usage_bytes: usize,
}

#[derive(Debug)]
pub struct ReceiveChannelUnreliable {
    channel_id: u8,
    messages: VecDeque<Bytes>,
    max_memory_usage_bytes: usize,
    memory_usage_bytes: usize,
}

impl SendChannelUnreliable {
    pub fn new(channel_id: u8, max_memory_usage_bytes: usize) -> Self {
        Self {
            channel_id,
            unreliable_messages: VecDeque::new(),
            max_memory_usage_bytes,
            memory_usage_bytes: 0,
        }
    }

    pub fn can_send_message(&self, size_bytes: usize) -> bool {
        size_bytes + self.memory_usage_bytes <= self.max_memory_usage_bytes
    }

    pub fn available_memory(&self) -> usize {
        self.max_memory_usage_bytes - self.memory_usage_bytes
    }

    pub fn get_packets_to_send(&mut self, available_bytes: &mut u64) -> Vec<Packet> {
        let mut packets: Vec<Packet> = vec![];
        let mut small_messages: Vec<Bytes> = vec![];
        let mut small_messages_bytes = 0;

        while let Some(message) = self.unreliable_messages.pop_front() {
            self.memory_usage_bytes -= message.len();
            if *available_bytes < message.len() as u64 {
                // Drop message, no available bytes to send
                continue;
            }

            *available_bytes -= message.len() as u64;

            let serialized_size = message.len() + octets::varint_len(message.len() as u64);
            if small_messages_bytes + serialized_size > MAX_MESSAGES_LENGTH {
                packets.push(Packet::SmallUnreliable {
                    channel_id: self.channel_id,
                    messages: std::mem::take(&mut small_messages),
                });
                small_messages_bytes = 0;
            }

            small_messages_bytes += serialized_size;
            small_messages.push(message);
        }

        // Generate final packet for remaining small messages
        if !small_messages.is_empty() {
            packets.push(Packet::SmallUnreliable {
                channel_id: self.channel_id,
                messages: std::mem::take(&mut small_messages),
            });
        }

        packets
    }

    pub fn send_message(&mut self, message: Bytes) {
        if self.memory_usage_bytes + message.len() > self.max_memory_usage_bytes {
            tracing::warn!(
                "dropped unreliable message sent because channel {} is memory limited",
                self.channel_id
            );
            return;
        }

        let message_size = message.len();
        if message_size > MAX_MESSAGES_LENGTH {
            tracing::error!(
                "Sending a message that is longer than {MAX_MESSAGES_LENGTH} is prohibited. Attempted message size: {message_size} ");
        }

        self.memory_usage_bytes += message.len();
        self.unreliable_messages.push_back(message);
    }
}

impl ReceiveChannelUnreliable {
    pub fn new(channel_id: u8, max_memory_usage_bytes: usize) -> Self {
        Self {
            channel_id,
            messages: VecDeque::new(),
            memory_usage_bytes: 0,
            max_memory_usage_bytes,
        }
    }

    pub fn process_message(&mut self, message: Bytes) {
        if self.memory_usage_bytes + message.len() > self.max_memory_usage_bytes {
            tracing::warn!(
                "dropped unreliable message received because channel {} is memory limited",
                self.channel_id
            );
            return;
        }

        self.memory_usage_bytes += message.len();
        self.messages.push_back(message);
    }

    pub fn receive_message(&mut self) -> Option<Bytes> {
        if let Some(message) = self.messages.pop_front() {
            self.memory_usage_bytes -= message.len();
            return Some(message);
        };

        None
    }
}
