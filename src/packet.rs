use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Packet {
    pub message_type: MessageType,
    pub raw: Vec<u8>, // Additional data for the packet
}

impl Packet {
    pub fn new(buffer: Vec<u8>) -> Packet {
        let message_type = MessageType::try_from(buffer[0]).unwrap();
        Packet {
            message_type,
            raw: buffer.to_vec(),
        }
    }
    pub fn get_connect_ack_raw(&self) -> Vec<u8> {
        let mut connect_ack = self.raw.clone();
        connect_ack[4] = 2;
        connect_ack
    }

    pub fn get_event_payload(&self) -> Vec<u8> {
        let remove_header = self.raw.iter().skip(10).cloned().collect();
        remove_header
    }
    pub fn get_message_header(&self) -> Vec<u8> {
        let message_type = MessageType::try_from(self.raw[0]).unwrap();
        match message_type {
            MessageType::Event => self.raw[1..9].to_vec(),
            MessageType::KeepAlive => self.raw[1..9].to_vec(),
            MessageType::Connect => self.raw[5..13].to_vec(),
            _ => {
                vec![]
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessageType {
    /// Client
    Connect = 85,
    Event = 1,
    Disconnect = 2,
    KeepAlive = 3,
    Other = 11,
}

impl TryFrom<u8> for MessageType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            85 => Ok(MessageType::Connect),
            1 => Ok(MessageType::Event),
            2 => Ok(MessageType::Disconnect),
            3 => Ok(MessageType::KeepAlive),
            _ => Ok(MessageType::Other),
        }
    }
}
