use bevy_ecs::{query::Changed, system::Query};
use bincode;
use serde::{Deserialize, Serialize};

use crate::game_state::{Player, Position, Rotation};

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

    pub fn position_event(positions: Vec<(Position, String)>) -> Option<EventOut> {
        let position_details: Vec<PositionDetails> = positions
            .iter()
            .map(|(position, player_id)| {
                let player_id_bytes = normalize_player_id(player_id.as_str());
                PositionDetails {
                    player_id: player_id_bytes,
                    position: position.clone(),
                }
            })
            .collect();

        if positions.len() > 0 {
            let position_event = PositionEventOut {
                positions: position_details,
            };
            tracing::info!("{:?}", position_event);

            let mut serialized = bincode::serialize(&position_event).unwrap();
            serialized.insert(0, 1); // Position Event Type 1
            return Some(EventOut {
                event_type: EventOutType::Position,
                data: serialized,
            });
        }
        None
    }

    pub fn rotation_event(
        query: &Query<(&Player, &Rotation), Changed<Rotation>>,
    ) -> Option<EventOut> {
        let rotations: Vec<RotationDetails> = query
            .iter()
            .map(|(player, rotation)| {
                let player_id_bytes = normalize_player_id(player.id.as_str());
                RotationDetails {
                    player_id: player_id_bytes,
                    rotation: rotation.clone(),
                }
            })
            .collect();

        if rotations.len() > 0 {
            let rotation_event = RotationEventOut { rotations };
            tracing::info!("{:?}", rotation_event);

            let mut serialized = bincode::serialize(&rotation_event).unwrap();
            serialized.insert(0, 2); // Rotation Event Type 1
            return Some(EventOut {
                event_type: EventOutType::Rotation,
                data: serialized,
            });
        }
        None
    }

    pub fn spawn_new_event(player_id: String, position: Position, rotation: Rotation) -> EventOut {
        let player_id_bytes = normalize_player_id(player_id.as_str());
        let spawns: Vec<SpawnDetails> = vec![SpawnDetails {
            player_id: player_id_bytes,
            position,
            rotation,
        }];

        let spawn_event = SpawnEventOut { spawns };

        let mut serialized = bincode::serialize(&spawn_event).unwrap();

        serialized.insert(0, 0); // Spawn Event Type 0

        EventOut {
            event_type: EventOutType::Spawn,
            data: serialized,
        }
    }

    pub fn spawn_event_for_all_players(
        query: &Query<(&Player, &Position, &Rotation)>,
    ) -> Option<EventOut> {
        let spawns: Vec<SpawnDetails> = query
            .iter()
            .map(|(player, position, rotation)| {
                let player_id_bytes = normalize_player_id(player.id.as_str());
                SpawnDetails {
                    player_id: player_id_bytes,
                    position: position.clone(),
                    rotation: rotation.clone(),
                }
            })
            .collect();
        if spawns.len() > 0 {
            let spawn_event = SpawnEventOut { spawns };

            let mut serialized = bincode::serialize(&spawn_event).unwrap();

            serialized.insert(0, 0); // Spawn Event Type 0

            return Some(EventOut {
                event_type: EventOutType::Spawn,
                data: serialized,
            });
        }
        None
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
    Rotation = 2,
    Disconnect = 10,
}

#[derive(Serialize, Deserialize, Debug)]
struct PositionEventOut {
    positions: Vec<PositionDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PositionDetails {
    player_id: [u8; 16],
    position: Position,
}
#[derive(Serialize, Deserialize, Debug)]
struct RotationEventOut {
    rotations: Vec<RotationDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RotationDetails {
    player_id: [u8; 16],
    rotation: Rotation,
}

#[derive(Serialize, Deserialize, Debug)]
struct SpawnEventOut {
    spawns: Vec<SpawnDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SpawnDetails {
    player_id: [u8; 16],
    position: Position,
    rotation: Rotation,
}

#[derive(Serialize, Deserialize, Debug)]
struct DisconnectEvent {
    disconnects: Vec<DisconnectDetails>,
}
#[derive(Serialize, Deserialize, Debug)]

struct DisconnectDetails {
    player_id: [u8; 16],
}
