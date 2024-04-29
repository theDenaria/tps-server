use std::{collections::HashMap, net::SocketAddr};

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

    pub fn add_player(&mut self, addr: SocketAddr, id: String) -> String {
        let player = Player::new(id.clone(), addr);
        self.players.insert(id.clone(), player); // Insert new player into HashMap
        id
    }

    pub fn get_player_mut(&mut self, id: String) -> Option<&mut Player> {
        self.players.get_mut(&id)
    }

    pub fn get_player(&self, id: String) -> Option<&Player> {
        self.players.get(&id)
    }

    pub fn all_players(&self) -> Vec<&Player> {
        self.players.values().collect()
    }
}

pub struct Player {
    pub id: String,
    pub connection_status: ConnectionStatus,
    pub addr: SocketAddr,
    pub position: PlayerPosition,
    speed: f32,
}

impl Player {
    fn new(id: String, addr: SocketAddr) -> Player {
        Player {
            id,
            connection_status: ConnectionStatus::Connecting,
            addr,
            position: PlayerPosition { x: 10.0, y: 10.0 },
            speed: 0.1,
        }
    }

    pub fn update_position(&mut self, move_input: MoveEvent) {
        self.position = PlayerPosition {
            x: self.position.x + (self.speed * move_input.x),
            y: self.position.y + (self.speed * move_input.y),
        }
    }

    pub fn set_connected(&mut self) {
        self.connection_status = ConnectionStatus::Connected;
    }
}

pub enum ConnectionStatus {
    Connecting = 0,
    Connected = 1,
    //TODO Disconnected = 2,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerPosition {
    pub x: f32,
    pub y: f32,
}
