use bincode;
use serde::{Deserialize, Serialize};

use crate::game_state::Player;

#[derive(Debug)]
pub struct EventOut {
    pub event_type: EventOutType,
    pub data: Vec<u8>,
}

impl EventOut {
    pub fn position_event(players: Vec<&Player>) -> Option<EventOut> {
        let player_num = players.len() as u32;
        if player_num < 1 {
            return None;
        }
        let mut positions: Vec<Position> = vec![];
        for player in players {
            let player_id_bytes = normalize_player_id(player.id.as_str());
            positions.push(Position {
                player_id: player_id_bytes,
                x: player.position.x,
                y: player.position.y,
            });
        }

        let position_event = PositionEvent { positions };

        let mut serialized = bincode::serialize(&position_event).unwrap();
        serialized.insert(0, 1); // Move Event Type 1
        Some(EventOut {
            event_type: EventOutType::Position,
            data: serialized,
        })
    }

    pub fn spawn_event(players: Vec<&Player>) -> Option<EventOut> {
        let player_num = players.len() as u32;
        if player_num < 1 {
            return None;
        }
        let mut positions: Vec<Position> = vec![];

        for player in players {
            let player_id_bytes = normalize_player_id(player.id.as_str());
            positions.push(Position {
                player_id: player_id_bytes,
                x: player.position.x,
                y: player.position.y,
            });
        }

        let spawn_event = PositionEvent { positions };

        let mut serialized = bincode::serialize(&spawn_event).unwrap();

        serialized.insert(0, 0); // Spawn Event Type 0

        Some(EventOut {
            event_type: EventOutType::Spawn,
            data: serialized,
        })
    }
}

fn normalize_player_id(player_id: &str) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    let player_id_bytes = player_id.as_bytes();
    let len = player_id_bytes.len().min(16);
    bytes[..len].copy_from_slice(&player_id_bytes[..len]);
    bytes
}

#[derive(Debug)]
pub enum EventOutType {
    Spawn = 0,
    Position = 1,
}

#[derive(Serialize, Deserialize, Debug)]
struct PositionEvent {
    positions: Vec<Position>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Position {
    player_id: [u8; 16],
    x: f32,
    y: f32,
}
