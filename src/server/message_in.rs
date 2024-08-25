use crate::ecs::events::{ConnectEvent, FireEvent, JumpEvent, LookEvent, MoveEvent};
use crate::server::packet::SerializationError;
use crate::sessions::SessionCreateInput;
use bevy::math::{Vec3, Vec4};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

#[derive(Debug)]
pub struct MessageIn {
    pub player_id: String,
    pub event_type: MessageInType,
    pub data: Vec<u8>,
}

impl MessageIn {
    pub fn new(player_id: String, bytes: Vec<u8>) -> Result<MessageIn, &'static str> {
        if bytes.len() < 1 {
            return Err("Not enough bytes for EventIn");
        }

        let event_type = MessageInType::try_from(bytes[0]).map_err(|_| "Invalid event type")?;
        let data = &bytes[1..];

        Ok(MessageIn {
            player_id,
            event_type,
            data: data.to_vec(),
        })
    }

    pub fn to_move_event(&self) -> Result<MoveEvent, SerializationError> {
        // let data_slice: &[u8] = &self.data;
        if self.data.len() < 8 {
            println!("Insufficent bytes: {:?}", self.data);
            return Err(SerializationError::BufferTooShort);
        }
        let mut reader = Cursor::new(&self.data);

        let x = reader.read_f32::<LittleEndian>()?;
        let y = reader.read_f32::<LittleEndian>()?;

        Ok(MoveEvent {
            player_id: self.player_id.clone(),
            x,
            y,
        })
    }
    pub fn to_look_event(&self) -> Result<LookEvent, SerializationError> {
        if self.data.len() < 12 {
            println!("Insufficent bytes: {:?}", self.data);
            return Err(SerializationError::BufferTooShort);
        }
        let mut reader = Cursor::new(&self.data);

        let x = reader.read_f32::<LittleEndian>()?;
        let y = reader.read_f32::<LittleEndian>()?;
        let z = reader.read_f32::<LittleEndian>()?;
        let w = reader.read_f32::<LittleEndian>()?;

        Ok(LookEvent {
            player_id: self.player_id.clone(),
            direction: Vec4::new(x, y, z, w),
        })
    }

    pub fn to_jump_event(&self) -> Result<JumpEvent, SerializationError> {
        if self.data.len() < 12 {
            println!("Insufficent bytes: {:?}", self.data);
            return Err(SerializationError::BufferTooShort);
        }

        Ok(JumpEvent {
            player_id: self.player_id.clone(),
        })
    }

    pub fn to_connect_event(&self) -> Result<ConnectEvent, SerializationError> {
        if self.data.len() < 1 {
            println!("Insufficent bytes: {:?}", self.data);
            return Err(SerializationError::BufferTooShort);
        }

        let _message =
            String::from_utf8(self.data.clone()).map_err(|_| SerializationError::CursorReadError);

        Ok(ConnectEvent {
            player_id: self.player_id.clone(),
        })
    }
    pub fn to_fire_event(&self) -> Result<FireEvent, SerializationError> {
        if self.data.len() < 8 {
            println!("Insufficent bytes: {:?}", self.data);
            return Err(SerializationError::BufferTooShort);
        }
        let mut reader = Cursor::new(&self.data);

        let cam_origin_x = reader.read_f32::<LittleEndian>()?;
        let cam_origin_y = reader.read_f32::<LittleEndian>()?;
        let cam_origin_z = reader.read_f32::<LittleEndian>()?;

        let direction_x = reader.read_f32::<LittleEndian>()?;
        let direction_y = reader.read_f32::<LittleEndian>()?;
        let direction_z = reader.read_f32::<LittleEndian>()?;

        let barrel_origin_x = reader.read_f32::<LittleEndian>()?;
        let barrel_origin_y = reader.read_f32::<LittleEndian>()?;
        let barrel_origin_z = reader.read_f32::<LittleEndian>()?;

        let cam_origin = Vec3::new(cam_origin_x, cam_origin_y, cam_origin_z);
        let direction = Vec3::new(direction_x, direction_y, direction_z);
        let barrel_origin = Vec3::new(barrel_origin_x, barrel_origin_y, barrel_origin_z);

        Ok(FireEvent {
            player_id: self.player_id.clone(),
            cam_origin,
            direction,
            barrel_origin,
        })
    }

    pub fn to_session_create_input(&self) -> Result<SessionCreateInput, SerializationError> {
        if self.data.len() < 10 {
            println!("Insufficent bytes: {:?}", self.data);
            return Err(SerializationError::BufferTooShort);
        }
        let mut reader = Cursor::new(&self.data);

        let id = reader.read_u64::<LittleEndian>()?;

        let players_len = reader.read_u16::<LittleEndian>()?;

        let mut players: Vec<(String, u8)> = vec![];

        for _ in 0..players_len {
            let mut player_id = String::new();
            reader.read_to_string(&mut player_id)?;
            let team = reader.read_u8()?;
            players.push((player_id, team));
        }
        Ok(SessionCreateInput { id, players })
    }
}

#[derive(Debug)]
pub enum MessageInType {
    Connect = 0,
    Move = 2,
    Rotation = 3,
    Jump = 4,
    Fire = 5,
    Invalid = 99,
    SessionCreate = 100,
    // SessionJoin = 101,
}

impl TryFrom<u8> for MessageInType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MessageInType::Connect),
            2 => Ok(MessageInType::Move),
            3 => Ok(MessageInType::Rotation),
            4 => Ok(MessageInType::Jump),
            5 => Ok(MessageInType::Fire),
            _ => Ok(MessageInType::Invalid),
        }
    }
}
