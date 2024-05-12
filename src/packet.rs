use bytes::Bytes;
use std::fmt;

pub type Payload = Vec<u8>;

enum NetworkEventType {}

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    // Small messages in a reliable channel are aggregated and sent in this packet
    SmallReliable {
        channel_id: u8,
        packet_type: u16,
        packet_process_time: u16,
        sequence_id: u16,
        acked_seq_id: u16,
        acked_mask: u32,
        messages: Vec<(u64, Bytes)>,
    },
    // Small messages in a unreliable channel are aggregated and sent in this packet
    SmallUnreliable {
        channel_id: u8,
        messages: Vec<Bytes>,
    },
    // Contains the packets that were acked
    // Acks are saved in multiples ranges, all values in the ranges are considered acked.
    Ack {
        channel_id: u8,
        packet_type: u16,
        packet_process_time: u16,
        sequence_id: u16,
        acked_seq_id: u16,
        acked_mask: u32,
        end_posfix: u8,
    },
}

impl Packet {
    pub fn sequence_id(&self) -> u16 {
        match self {
            Packet::SmallReliable { sequence_id, .. } => *sequence_id,
            Packet::SmallUnreliable { .. } => 0, // Return 0 when there's no sequence_id
            Packet::Ack { sequence_id, .. } => *sequence_id,
        }
    }

    pub fn to_bytes(&self, b: &mut octets::OctetsMut) -> Result<usize, SerializationError> {
        let before = b.cap();

        match self {
            Packet::SmallReliable {
                channel_id,
                packet_type,
                packet_process_time,
                sequence_id,
                acked_seq_id,
                acked_mask,
                messages,
            } => {
                b.put_u8(*channel_id)?;
                b.put_u16(*packet_type)?;
                b.put_u16(*packet_process_time)?;
                b.put_u16(*sequence_id)?;
                b.put_u16(*acked_seq_id)?;
                b.put_u32(*acked_mask)?;
                b.put_u16(messages.len() as u16)?;
                for (message_id, message) in messages {
                    b.put_varint(*message_id)?;
                    b.put_varint(message.len() as u64)?;
                    b.put_bytes(message)?;
                }
            }
            Packet::SmallUnreliable {
                channel_id,
                messages,
            } => {
                b.put_u8(*channel_id)?;
                b.put_u16(messages.len() as u16)?;
                for message in messages {
                    b.put_varint(message.len() as u64)?;
                    b.put_bytes(message)?;
                }
            }
            Packet::Ack {
                channel_id,
                packet_type,
                packet_process_time,
                sequence_id,
                acked_seq_id,
                acked_mask,
                end_posfix,
            } => {
                b.put_u8(*channel_id)?;
                b.put_u16(*packet_type)?;
                b.put_u16(*packet_process_time)?;
                b.put_u16(*sequence_id)?;
                b.put_u16(*acked_seq_id)?;
                b.put_u32(*acked_mask)?;
                b.put_u8(*end_posfix)?;
            }
        }

        Ok(before - b.cap())
    }

    pub fn from_bytes(b: &mut octets::Octets) -> Result<Packet, SerializationError> {
        // let packet_type = b.get_u8()?;
        let channel_id = b.get_u8()?;

        let mut messages: Vec<Bytes> = Vec::with_capacity(64);
        match channel_id {
            0 => {
                // SmallUnreliable
                let messages_len = b.get_u16()?;
                for _ in 0..messages_len {
                    let payload = b.get_bytes_with_varint_length()?;
                    messages.push(payload.to_vec().into());
                }
                Ok(Packet::SmallUnreliable {
                    channel_id,
                    messages,
                })
            }

            1 => {
                let packet_type = b.get_u16()?;
                let packet_process_time = b.get_u16()?;
                let sequence_id = b.get_u16()?;
                let acked_seq_id = b.get_u16()?;
                let acked_mask = b.get_u32()?;
                match packet_type {
                    0 => {
                        // SmallReliable Payload
                        let messages_len = b.get_u16()?;
                        let mut messages: Vec<(u64, Bytes)> = Vec::with_capacity(64);
                        for _ in 0..messages_len {
                            let message_id = b.get_varint()?;
                            let payload = b.get_bytes_with_varint_length()?;

                            messages.push((message_id, payload.to_vec().into()));
                        }
                        Ok(Packet::SmallReliable {
                            channel_id,
                            packet_type,
                            packet_process_time,
                            sequence_id,
                            acked_seq_id,
                            acked_mask,
                            messages,
                        })
                    }
                    1 => {
                        // SmallReliable Ack
                        let end_posfix = b.get_u8()?;
                        Ok(Packet::Ack {
                            channel_id,
                            packet_type,
                            packet_process_time,
                            sequence_id,
                            acked_seq_id,
                            acked_mask,
                            end_posfix,
                        })
                    }
                    _ => Err(SerializationError::InvalidPacketType),
                }
            }
            _ => Err(SerializationError::InvalidChannelId),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationError {
    BufferTooShort,
    InvalidNumSlices,
    InvalidAckRange,
    InvalidPacketType,
    InvalidChannelId,
}

impl std::error::Error for SerializationError {}

impl fmt::Display for SerializationError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use SerializationError::*;

        match *self {
            BufferTooShort => write!(fmt, "buffer too short"),
            InvalidNumSlices => write!(fmt, "invalid number of slices"),
            InvalidAckRange => write!(fmt, "invalid ack range"),
            InvalidPacketType => write!(fmt, "invalid packet type"),
            InvalidChannelId => write!(fmt, "invalid channel id"),
        }
    }
}

impl From<octets::BufferTooShortError> for SerializationError {
    fn from(_: octets::BufferTooShortError) -> Self {
        SerializationError::BufferTooShort
    }
}
