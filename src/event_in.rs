#[derive(Debug)]
pub struct EventIn {
    pub event_type: EventInType,
    pub data: Vec<u8>,
}

impl EventIn {
    pub fn new(bytes: Vec<u8>) -> Result<EventIn, &'static str> {
        if bytes.len() < 2 {
            return Err("Not enough bytes for EventIn");
        }

        let event_type = EventInType::try_from(bytes[0]).map_err(|_| "Invalid event type")?;
        let data = &bytes[1..];

        Ok(EventIn {
            event_type,
            data: data.to_vec(),
        })
    }
}

#[derive(Debug)]
pub enum EventInType {
    Connect = 0,
    Move = 2,
    Rotation = 3,
    Invalid = 99,
}

#[derive(Debug)]
pub struct MoveEvent {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub struct ConnectEvent {
    pub message: String,
}

#[derive(Debug)]
pub struct DisconnectEvent {
    pub message: String,
}

impl TryFrom<u8> for EventInType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EventInType::Connect),
            2 => Ok(EventInType::Move),
            3 => Ok(EventInType::Rotation),
            _ => Ok(EventInType::Invalid),
        }
    }
}

pub fn digest_move_event(data: Vec<u8>) -> Result<MoveEvent, &'static str> {
    if data.len() < 8 {
        println!("Insufficent bytes: {:?}", data);
        return Err("Insufficient bytes for MoveInputUpdate");
    }

    let x_bytes = data[0..4]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;
    let y_bytes = data[4..8]
        .try_into()
        .map_err(|_| "Failed to slice y bytes")?;

    let x = f32::from_ne_bytes(x_bytes);
    let y = f32::from_ne_bytes(y_bytes);

    Ok(MoveEvent { x, y })
}

pub fn digest_rotation_event(data: Vec<u8>) -> Result<f32, &'static str> {
    if data.len() < 4 {
        println!("Insufficent bytes: {:?}", data);
        return Err("Insufficient bytes for Rotation");
    }

    let rotation_bytes = data[0..4]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;

    let rotation = f32::from_ne_bytes(rotation_bytes);

    Ok(rotation)
}

pub fn digest_connect_event(data: Vec<u8>) -> Result<ConnectEvent, &'static str> {
    if data.len() < 1 {
        return Err("Insufficient bytes for Connect");
    }

    let message = String::from_utf8(data).map_err(|_| "Invalid UTF-8 in player_id")?;

    Ok(ConnectEvent { message })
}
