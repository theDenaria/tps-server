use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use bytes::{Buf, Bytes};
use std::{
    fmt::{self},
    io::{Cursor, Read, Write},
};
pub type Payload = Vec<u8>;

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

    pub fn to_bytes(&self, b: &mut [u8]) -> Result<usize, SerializationError> {
        let mut writer = Cursor::new(b);
        let before = writer.remaining();
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
                writer.write_u8(*channel_id)?;
                writer.write_u16::<LittleEndian>(*packet_type)?;
                writer.write_u16::<LittleEndian>(*packet_process_time)?;
                writer.write_u16::<LittleEndian>(*sequence_id)?;
                writer.write_u16::<LittleEndian>(*acked_seq_id)?;
                writer.write_u32::<LittleEndian>(*acked_mask)?;
                writer.write_u16::<LittleEndian>(messages.len() as u16)?;
                for (message_id, message) in messages {
                    writer.write_u64::<LittleEndian>(*message_id)?;
                    writer.write_u16::<LittleEndian>(message.len() as u16)?;
                    writer.write_all(message)?;
                }
            }
            Packet::SmallUnreliable {
                channel_id,
                messages,
            } => {
                writer.write_u8(*channel_id)?;
                writer.write_u16::<LittleEndian>(messages.len() as u16)?;
                for message in messages {
                    writer.write_u16::<LittleEndian>(message.len() as u16)?;
                    writer.write_all(message)?;
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
                writer.write_u8(*channel_id)?;
                writer.write_u16::<LittleEndian>(*packet_type)?;
                writer.write_u16::<LittleEndian>(*packet_process_time)?;
                writer.write_u16::<LittleEndian>(*sequence_id)?;
                writer.write_u16::<LittleEndian>(*acked_seq_id)?;
                writer.write_u32::<LittleEndian>(*acked_mask)?;
                writer.write_u8(*end_posfix)?;
            }
        }

        Ok(before - writer.remaining())
    }

    pub fn from_bytes(b: &[u8]) -> Result<Packet, SerializationError> {
        let mut reader = Cursor::new(b);
        let channel_id = reader.read_u8()?;
        let mut messages: Vec<Bytes> = Vec::with_capacity(64);
        match channel_id {
            0 => {
                // SmallUnreliable
                let messages_len = reader.read_u16::<LittleEndian>()?;
                for _ in 0..messages_len {
                    let message_len = reader.read_u16::<LittleEndian>()?;
                    let mut data = vec![0u8; message_len as usize];
                    reader.read_exact(&mut data)?;
                    messages.push(data.into());
                }
                Ok(Packet::SmallUnreliable {
                    channel_id,
                    messages,
                })
            }

            1 => {
                let packet_type = reader.read_u16::<LittleEndian>()?;
                let packet_process_time = reader.read_u16::<LittleEndian>()?;
                let sequence_id = reader.read_u16::<LittleEndian>()?;
                let acked_seq_id = reader.read_u16::<LittleEndian>()?;
                let acked_mask = reader.read_u32::<LittleEndian>()?;
                match packet_type {
                    0 => {
                        // SmallReliable Payload
                        let messages_len = reader.read_u16::<LittleEndian>()?;
                        let mut messages: Vec<(u64, Bytes)> = Vec::with_capacity(64);
                        for _ in 0..messages_len {
                            let message_id = reader.read_u64::<LittleEndian>()?;
                            let message_len = reader.read_u16::<LittleEndian>()?;
                            let mut data = vec![0u8; message_len as usize];
                            reader.read_exact(&mut data)?;

                            messages.push((message_id, data.into()));
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
                        let end_posfix = reader.read_u8()?;
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
    #[allow(dead_code)]
    InvalidNumSlices,
    #[allow(dead_code)]
    InvalidAckRange,
    InvalidPacketType,
    InvalidChannelId,
    CursorReadError,
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
            CursorReadError => write!(fmt, "cursor read error"),
        }
    }
}

impl From<std::io::Error> for SerializationError {
    fn from(error: std::io::Error) -> Self {
        tracing::error!("IN ERROR FROM: {:?}", error);
        SerializationError::CursorReadError
    }
}
