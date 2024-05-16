use bincode;
use serde::{Deserialize, Serialize};

use crate::game_state::Player;

#[derive(Debug)]
pub struct EventOut {
    pub event_type: EventOutType,
    pub data: Vec<u8>,
}

impl EventOut {
    pub fn get_with_event_header(&self, identifier: Vec<u8>) -> Vec<u8> {
        let mut with_header: Vec<u8> = vec![];
        with_header.push(1);

        with_header.extend(identifier);
        with_header.push(0);
        with_header.extend(self.data.clone());
        with_header
    }

    pub fn position_event(players: Vec<&Player>) -> Option<EventOut> {
        let player_num = players.len() as u32;
        if player_num < 1 {
            return None;
        }
        let mut positions: Vec<Position> = vec![];
        for player in players {
            if player.position.updated {
                let player_id_bytes = normalize_player_id(player.id.as_str());
                positions.push(Position {
                    player_id: player_id_bytes,
                    x: player.position.x,
                    y: player.position.y,
                    rotation: player.rotation,
                });
            }
        }
        if positions.len() > 0 {
            let position_event = PositionEvent { positions };
            tracing::info!("{:?}", position_event);

            let mut serialized = bincode::serialize(&position_event).unwrap();
            serialized.insert(0, 1); // Move Event Type 1
            return Some(EventOut {
                event_type: EventOutType::Position,
                data: serialized,
            });
        }
        None
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
                rotation: player.rotation,
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

    pub fn spawn_event_by_player_id(player_id: &String) -> EventOut {
        let mut positions: Vec<Position> = vec![];

        let player_id_bytes = normalize_player_id(player_id.as_str());
        positions.push(Position {
            player_id: player_id_bytes,
            x: 10.0,
            y: 10.0,
            rotation: 0.0,
        });

        let spawn_event = PositionEvent { positions };

        let mut serialized = bincode::serialize(&spawn_event).unwrap();

        serialized.insert(0, 0); // Spawn Event Type 0

        EventOut {
            event_type: EventOutType::Spawn,
            data: serialized,
        }
    }
    pub fn disconnect_event(player_ids: Vec<&String>) -> Option<EventOut> {
        let player_num = player_ids.len() as u32;
        if player_num < 1 {
            return None;
        }
        let mut disconnects: Vec<DisconnectDetails> = vec![];

        for player_id in player_ids {
            let player_id_bytes = normalize_player_id(player_id.as_str());
            disconnects.push(DisconnectDetails {
                player_id: player_id_bytes,
            });
        }

        let disconnect_event = DisconnectEvent { disconnects };

        let mut serialized = bincode::serialize(&disconnect_event).unwrap();

        serialized.insert(0, 10); // Disconnect Event Type 10

        Some(EventOut {
            event_type: EventOutType::Disconnect,
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
    Disconnect = 10,
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
    rotation: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct DisconnectEvent {
    disconnects: Vec<DisconnectDetails>,
}
#[derive(Serialize, Deserialize, Debug)]

struct DisconnectDetails {
    player_id: [u8; 16],
}
