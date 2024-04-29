use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Mutex};

use crate::event_in::{digest_connect_event, digest_move_event, EventIn, EventInType};
use crate::event_out::EventOut;
use crate::game_state::{ConnectionStatus, GameState, Player};
use crate::packet::{MessageType, Packet};

pub struct Server {
    tick_rate: f32,
    socket: Arc<Mutex<UdpSocket>>,
    game_state: Arc<Mutex<GameState>>,
    connection_handler: Arc<Mutex<ConnectionHandler>>,
}

impl Server {
    pub async fn new(options: Option<ServerOptions>) -> Server {
        let options = options.unwrap_or(ServerOptions { tick_rate: 60.0 });
        let socket = UdpSocket::bind("0.0.0.0:5000").await.unwrap();
        Server {
            tick_rate: options.tick_rate,
            game_state: Arc::new(Mutex::new(GameState::new())),
            socket: Arc::new(Mutex::new(socket)),
            connection_handler: Arc::new(Mutex::new(ConnectionHandler::new())),
        }
    }

    pub async fn start(self: Arc<Self>) {
        let (tx, mut rx) = mpsc::channel(100);

        let self_clone = self.clone();

        self_clone.run_receiver(tx).await;

        // Main game loop in a separate thread
        thread::spawn(move || {
            let tick_duration = Duration::from_secs_f64(1.0 / self.tick_rate as f64);
            let mut next_tick = Instant::now() + tick_duration;
            loop {
                let start_time = Instant::now();

                // Process all available messages
                while let Ok((message, addr)) = rx.try_recv() {
                    if message.len() > 0 {
                        let self_clone = self.clone();
                        tokio::runtime::Runtime::new().unwrap().block_on(async {
                            self_clone.process_client_messages(message, addr).await;
                        });
                    }
                }

                // Update game logic
                let self_clone = self.clone();
                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    self_clone.send_game_state_updates().await;
                });

                // Calculate the remaining time until the next tick
                let elapsed = Instant::now().duration_since(start_time);
                if elapsed < tick_duration {
                    next_tick += tick_duration;
                    let sleep_duration = next_tick - Instant::now();
                    if sleep_duration > Duration::from_millis(0) {
                        thread::sleep(sleep_duration);
                    }
                } else {
                    // We're running behind, skip to the next tick
                    next_tick = Instant::now() + tick_duration;
                }
            }
        })
        .join()
        .unwrap();
    }

    async fn process_client_messages(&self, message: Vec<u8>, addr: SocketAddr) {
        let packet = Packet::new(message);
        let socket = self.socket.clone();
        let game_state = self.game_state.clone();
        let connection_handler = self.connection_handler.clone();

        match packet.message_type {
            MessageType::Event => {
                let payload = packet.get_event_payload();
                let event = EventIn::new(payload).unwrap();
                println!("Received: {:?}", event);
                match event.event_type {
                    EventInType::Move => {
                        let move_input = digest_move_event(event.data).unwrap();
                        let mut game_state = game_state.lock().await;
                        if let Some(player) = game_state.get_player_mut(event.player_id) {
                            player.update_position(move_input);
                        }
                    }
                    EventInType::Connect => {
                        let packet_header = packet.get_event_header().clone();
                        let _connect_event = digest_connect_event(event.data).unwrap();
                        let player_id = event.player_id;

                        let is_new = connection_handler
                            .lock()
                            .await
                            .new_connection(&player_id, packet_header)
                            .await;
                        // This means already added this player
                        let mut game_state = game_state.lock().await;
                        if is_new {
                            game_state.add_player(addr, player_id);
                        }
                    }
                    EventInType::ConnectConfirm => {
                        let _connect_event = digest_connect_event(event.data).unwrap();
                        let player_id = event.player_id;

                        // This means already added this player
                        let mut game_state = game_state.lock().await;
                        match game_state.get_player_mut(player_id) {
                            Some(player) => {
                                player.set_connected();
                            }
                            None => {
                                panic!("No player found for Connect Confirm");
                            }
                        }
                    }
                    EventInType::Invalid => {}
                }
            }
            MessageType::Connect => {
                socket
                    .lock()
                    .await
                    .send_to(packet.raw.as_slice(), addr)
                    .await
                    .unwrap();
                println!("{:?} Connect ack bytes sent", packet);
            }
            MessageType::KeepAlive => {
                socket
                    .lock()
                    .await
                    .send_to(packet.raw.as_slice(), addr)
                    .await
                    .unwrap();
            }
            _ => {}
        }
    }

    async fn send_game_state_updates(&self) {
        let socket = self.socket.clone();
        let game_state = self.game_state.lock().await;
        let mut connection_handler = self.connection_handler.lock().await; //clone();

        let position_event = EventOut::position_event(game_state.all_players());

        let mut pending_players: Vec<&Player> = vec![];
        for connection in connection_handler.pending.lock().await.values() {
            match game_state.get_player(connection.player_id.clone()) {
                Some(pl) => pending_players.push(pl),
                None => {}
            }
        }
        let spawn_event = EventOut::spawn_event(pending_players);
        let players = game_state.all_players();
        for player in players {
            match player.connection_status {
                ConnectionStatus::Connected => {
                    let identifier = connection_handler
                        .connected
                        .lock()
                        .await
                        .get(&player.id)
                        .unwrap()
                        .identifier
                        .clone();
                    if let Some(ref pe) = position_event {
                        let mut packet = identifier.clone();
                        packet.extend(pe.data.clone());
                        println!("Sent: {:?}", pe);
                        socket
                            .lock()
                            .await
                            .send_to(packet.as_slice(), player.addr)
                            .await
                            .unwrap();
                    }
                    if let Some(ref se) = spawn_event {
                        println!("Sent : {:?}", se);
                        let mut packet = identifier.clone();
                        packet.extend(se.data.clone());
                        socket
                            .lock()
                            .await
                            .send_to(packet.as_slice(), player.addr)
                            .await
                            .unwrap();
                    }
                }
                ConnectionStatus::Connecting => {
                    let maybe_identifier = {
                        let guard = connection_handler.pending.lock().await;
                        guard.get(&player.id).map(|info| info.identifier.clone())
                    };

                    if let Some(identifier) = maybe_identifier {
                        if let Some(ref se) = spawn_event {
                            println!("Sent : {:?}", se);
                            let mut packet = identifier.clone();
                            packet.extend(se.data.clone());
                            socket
                                .lock()
                                .await
                                .send_to(packet.as_slice(), player.addr)
                                .await
                                .unwrap();
                        }

                        // Now that the lock guard is dropped, we can perform a mutable borrow
                        connection_handler.set_connected(&player.id).await;
                    }
                } //TODO ConnectionStatus::Disconnected => {}
            }
        }
    }

    pub async fn run_receiver(self: Arc<Self>, tx: Sender<(Vec<u8>, SocketAddr)>) {
        // Clone `self` for the asynchronous task to listen for UDP messages
        let self_clone = self.clone();
        tokio::spawn(async move {
            let socket = self_clone.socket.clone();
            let mut buf = vec![0u8; 1024];
            loop {
                // Scoping the lock so it gets dropped before the await
                let (len, addr) = {
                    let socket_lock = socket.lock().await;
                    socket_lock.recv_from(&mut buf).await.unwrap()
                }; // The lock is released here because the scope ends

                let msg = buf[..len].to_vec();
                tx.send((msg, addr)).await.unwrap(); // Send message to game logic
            }
        });
    }
}

pub struct ServerOptions {
    tick_rate: f32,
}

struct ConnectionHandler {
    pub pending: Mutex<HashMap<String, Connection>>,
    pub connected: Mutex<HashMap<String, Connection>>,
}

impl ConnectionHandler {
    pub fn new() -> ConnectionHandler {
        ConnectionHandler {
            pending: Mutex::new(HashMap::new()),
            connected: Mutex::new(HashMap::new()),
        }
    }
    /// Returns false if this connection as is already initialized
    pub async fn new_connection(&mut self, player_id: &String, identifier: Vec<u8>) -> bool {
        let mut pending = self.pending.lock().await;
        if let Some(_) = pending.get(player_id) {
            return false;
        }
        let new_connection = Connection {
            identifier: identifier.clone(),
            player_id: player_id.clone(),
        };
        pending.insert(player_id.clone(), new_connection);
        true
    }

    pub async fn set_connected(&mut self, player_id: &String) -> bool {
        let mut pending = self.pending.lock().await;

        match pending.get(player_id) {
            Some(pend) => {
                let mut connected = self.connected.lock().await;
                let new_connection = Connection {
                    identifier: pend.identifier.clone(),
                    player_id: player_id.clone(),
                };
                connected.insert(player_id.clone(), new_connection);
                pending.remove(player_id);

                true
            }
            None => false,
        }
    }
}

struct Connection {
    pub identifier: Vec<u8>,
    pub player_id: String,
}
