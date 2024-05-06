use std::{collections::HashMap, time::Duration};

use tokio::time::Instant;

static CONNECTION_TIMEOUT_SECS: u64 = 5;

pub struct ConnectionHandler {
    pub pending_index: ConnectionIndex,
    pub connected_index: ConnectionIndex,
    pub disconnected_index: Vec<String>,
}

impl ConnectionHandler {
    pub fn new() -> ConnectionHandler {
        ConnectionHandler {
            pending_index: ConnectionIndex::new(),
            connected_index: ConnectionIndex::new(),
            disconnected_index: vec![],
        }
        .into()
    }
    /// Returns false if this connection as is already initialized
    pub fn new_connection(&mut self, player_id: &String, identifier: Vec<u8>) -> bool {
        if self
            .pending_index
            .get_connection(Some(player_id), None)
            .is_some()
        {
            return false;
        }

        let new_connection = Connection {
            identifier: identifier.clone(),
            player_id: player_id.clone(),
            last_message_time: Instant::now(),
        };

        self.pending_index.add_connection(&new_connection);
        true
    }

    pub fn set_connected(&mut self, player_id: &String) -> bool {
        if self
            .pending_index
            .get_connection(Some(player_id), None)
            .is_none()
        {
            return false;
        }

        match self.pending_index.get_connection(Some(player_id), None) {
            Some(pend) => {
                let new_connection = Connection {
                    identifier: pend.identifier.clone(),
                    player_id: pend.player_id.clone(),
                    last_message_time: Instant::now(),
                };

                self.connected_index.add_connection(&new_connection);
                self.pending_index.remove_connection(Some(player_id), None);
                true
            }
            None => false,
        }
    }

    pub fn set_disconnected(&mut self, player_id: &String) {
        self.connected_index
            .remove_connection(Some(player_id), None);
        self.disconnected_index.push(player_id.clone());
    }

    pub fn set_last_message_time(&mut self, identifier: Vec<u8>) {
        match self.get_connected_connection(None, Some(identifier.clone())) {
            Some(con) => self
                .connected_index
                .update_last_message_time(con.identifier),
            None => match self.get_pending_connection(None, Some(identifier)) {
                Some(con) => self.pending_index.update_last_message_time(con.identifier),
                None => return,
            },
        }
    }

    pub fn check_timeout(&mut self) -> Vec<String> {
        let mut timed_out_players: Vec<String> = vec![];
        let time_now = Instant::now();
        for con in self.get_connected_connections() {
            if time_now.duration_since(con.last_message_time)
                > Duration::from_secs(CONNECTION_TIMEOUT_SECS)
            {
                self.connected_index
                    .remove_connection(Some(&con.player_id), None);
                timed_out_players.push(con.player_id.clone());
                self.disconnected_index.push(con.player_id.clone());
                tracing::info!("Player: {:?} connection has timed out", con.player_id);
            }
        }
        for con in self.get_pending_connections() {
            if time_now.duration_since(con.last_message_time)
                > Duration::from_secs(CONNECTION_TIMEOUT_SECS)
            {
                self.connected_index
                    .remove_connection(Some(&con.player_id), None);
                timed_out_players.push(con.player_id.clone());
                self.disconnected_index.push(con.player_id.clone());
                tracing::info!(
                    "Player: {:?} pending connection has timed out",
                    con.player_id
                );
            }
        }
        timed_out_players
    }

    pub fn get_pending_connections(&self) -> Vec<Connection> {
        self.pending_index.by_player_id.values().cloned().collect()
    }
    pub fn get_connected_connections(&self) -> Vec<Connection> {
        self.connected_index
            .by_player_id
            .values()
            .cloned()
            .collect()
    }

    pub fn get_disconnected_player_ids(&self) -> Vec<String> {
        self.disconnected_index.clone()
    }

    pub fn clean_disconnected_list(&mut self) {
        self.disconnected_index = vec![];
    }

    pub fn get_pending_connection(
        &self,
        player_id: Option<&String>,
        identifier: Option<Vec<u8>>,
    ) -> Option<Connection> {
        self.pending_index.get_connection(player_id, identifier)
    }
    pub fn get_connected_connection(
        &self,
        player_id: Option<&String>,
        identifier: Option<Vec<u8>>,
    ) -> Option<Connection> {
        self.connected_index.get_connection(player_id, identifier)
    }

    pub fn get_connected_identifier(&self, player_id: &String) -> Option<Vec<u8>> {
        match self.get_connected_connection(Some(player_id), None) {
            Some(con) => {
                return Some(con.identifier.clone());
            }
            None => None,
        }
    }

    pub fn get_pending_identifier(&self, player_id: &String) -> Option<Vec<u8>> {
        match self.get_pending_connection(Some(player_id), None) {
            Some(con) => Some(con.identifier.clone()),
            None => None,
        }
    }

    pub fn get_connected_player_id(&self, identifier: Vec<u8>) -> Option<String> {
        match self.get_connected_connection(None, Some(identifier)) {
            Some(con) => {
                return Some(con.player_id);
            }
            None => None,
        }
    }

    pub fn get_pending_player_id(&self, identifier: Vec<u8>) -> Option<String> {
        match self.get_pending_connection(None, Some(identifier)) {
            Some(con) => Some(con.player_id),
            None => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Connection {
    pub identifier: Vec<u8>,
    pub player_id: String,
    pub last_message_time: Instant,
}

#[derive(Debug)]
pub struct ConnectionIndex {
    by_identifier: HashMap<Vec<u8>, Connection>,
    by_player_id: HashMap<String, Connection>,
}

impl ConnectionIndex {
    pub fn new() -> ConnectionIndex {
        ConnectionIndex {
            by_identifier: HashMap::new(),
            by_player_id: HashMap::new(),
        }
    }
    pub fn add_connection(&mut self, connection: &Connection) {
        self.by_identifier
            .insert(connection.identifier.clone(), connection.clone());
        self.by_player_id
            .insert(connection.player_id.clone(), connection.clone());
    }
    pub fn get_connection(
        &self,
        player_id: Option<&String>,
        identifier: Option<Vec<u8>>,
    ) -> Option<Connection> {
        match player_id {
            Some(pid) => {
                return self.by_player_id.get(pid).cloned();
            }
            None => match identifier {
                Some(id) => {
                    return self.by_identifier.get(&id).cloned();
                }
                None => None,
            },
        }
    }

    pub fn update_last_message_time(&mut self, identifier: Vec<u8>) {
        if let Some(con_by_id) = self.by_identifier.get_mut(&identifier) {
            let con_by_player = self.by_player_id.get_mut(&con_by_id.player_id).unwrap();
            let now = Instant::now();
            con_by_player.last_message_time = now;
            con_by_id.last_message_time = now;
        }
    }

    pub fn remove_connection(&mut self, player_id: Option<&String>, identifier: Option<Vec<u8>>) {
        let maybe_connection = self.get_connection(player_id, identifier);
        if let Some(connection) = maybe_connection {
            self.by_identifier.remove(&connection.identifier);
            self.by_player_id.remove(&connection.player_id);
        } else {
            tracing::error!("Unexpected no connection found!")
        }
    }
}
