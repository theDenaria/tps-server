use std::collections::HashMap;

use crate::event_in::MoveEvent;

pub struct GameState {
    pub players: HashMap<String, Player>,
}

impl GameState {
    pub fn new() -> GameState {
        GameState {
            players: HashMap::new(),
        }
    }

    pub fn add_player(&mut self, id: &String) {
        let player = Player::new(id.clone());
        self.players.insert(id.clone(), player); // Insert new player into HashMap
    }

    pub fn remove_player(&mut self, id: &String) {
        self.players.remove(id);
    }

    pub fn get_player_mut(&mut self, id: &String) -> Option<&mut Player> {
        self.players.get_mut(id)
    }

    pub fn get_player(&self, id: &String) -> Option<&Player> {
        self.players.get(id)
    }

    pub fn all_players_mut(&mut self) -> Vec<&mut Player> {
        self.players.values_mut().collect()
    }

    pub fn all_players(&self) -> Vec<&Player> {
        self.players.values().collect()
    }
}

pub struct Player {
    // Metadata
    pub id: String,
    // State attributes
    pub position: PlayerPosition,
    pub rotation: f32,
    speed: f32,
}

impl Player {
    fn new(id: String) -> Player {
        Player {
            id,
            position: PlayerPosition {
                updated: true,
                x: 10.0,
                y: 10.0,
            },
            rotation: 0.0,
            speed: 0.1,
        }
    }

    pub fn update_position(&mut self, move_input: MoveEvent) {
        self.position = PlayerPosition {
            updated: true,
            x: self.position.x + (self.speed * move_input.x),
            y: self.position.y + (self.speed * move_input.y),
        }
    }

    pub fn set_position_updated(&mut self, updated: bool) {
        self.position.updated = updated;
    }

    pub fn update_rotation(&mut self, rotation: f32) {
        self.rotation = rotation;
        self.set_position_updated(true);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerPosition {
    pub updated: bool,
    pub x: f32,
    pub y: f32,
}
