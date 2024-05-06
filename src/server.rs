use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Mutex};

use crate::connection_handler::ConnectionHandler;
use crate::event_in::{
    digest_connect_event, digest_disconnect_event, digest_move_event, digest_rotation_event,
    EventIn, EventInType,
};
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
                    if message.len() > 8 {
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
        let mut connection_handler = self.connection_handler.lock().await;
        let packet_identifier = packet.get_message_header().clone();
        connection_handler.set_last_message_time(packet_identifier.clone());

        match packet.message_type {
            MessageType::Event => {
                let payload = packet.get_event_payload();
                let event = EventIn::new(payload).unwrap();
                tracing::info!("Received: {:?}", event);
                match event.event_type {
                    EventInType::Rotation => {
                        let rotation_input = digest_rotation_event(event.data).unwrap();
                        let mut game_state = game_state.lock().await;
                        if let Some(player) = game_state.get_player_mut(event.player_id) {
                            player.update_rotation(rotation_input);
                        }
                    }
                    EventInType::Move => {
                        let move_input = digest_move_event(event.data).unwrap();
                        let mut game_state = game_state.lock().await;
                        if let Some(player) = game_state.get_player_mut(event.player_id) {
                            player.update_position(move_input);
                        }
                    }
                    EventInType::Connect => {
                        let _connect_event = digest_connect_event(event.data).unwrap();
                        let player_id = event.player_id;

                        let is_new =
                            connection_handler.new_connection(&player_id, packet_identifier);

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
                    EventInType::Disconnect => {
                        let disconnect_event = digest_disconnect_event(event.data).unwrap();

                        if let Some(player_id) =
                            connection_handler.get_connected_player_id(packet_identifier)
                        {
                            let mut game_state = game_state.lock().await;
                            game_state.remove_player(player_id.clone());
                            connection_handler.set_disconnected(&player_id);
                        }
                    }
                    EventInType::Invalid => {}
                }
            }
            MessageType::Connect => {
                socket
                    .lock()
                    .await
                    .send_to(packet.get_connect_ack_raw().as_slice(), addr)
                    .await
                    .unwrap();
                tracing::info!("{:?} Connect ack bytes sent", packet.get_connect_ack_raw());
            }
            MessageType::KeepAlive => {
                socket
                    .lock()
                    .await
                    .send_to(packet.raw.as_slice(), addr)
                    .await
                    .unwrap();
            }
            MessageType::Disconnect => {
                tracing::info!("DISCONNECT PACKET: {:?}", packet);
            }
            MessageType::Other => {
                tracing::info!("OTHER PACKET: {:?}", packet);
            }
        }
    }

    async fn send_game_state_updates(&self) {
        let socket = self.socket.clone();
        let mut game_state = self.game_state.lock().await;
        let mut connection_handler = self.connection_handler.lock().await;

        let timed_out_players = connection_handler.check_timeout();

        for player_id in timed_out_players {
            game_state.remove_player(player_id.clone());
        }

        let all_players_mut = game_state.all_players_mut();

        let position_event = EventOut::position_event(all_players_mut);

        let pending_players_connection = connection_handler.get_pending_connections();
        let mut pending_players: Vec<&Player> = vec![];

        for connection in pending_players_connection {
            match game_state.get_player(connection.player_id.clone()) {
                Some(pl) => pending_players.push(pl),
                None => {}
            }
        }

        let connected_players_connection = connection_handler.get_connected_connections();
        let mut connected_players: Vec<&Player> = vec![];

        for connection in connected_players_connection {
            match game_state.get_player(connection.player_id.clone()) {
                Some(pl) => connected_players.push(pl),
                None => {}
            }
        }

        let spawn_event = EventOut::spawn_event(pending_players);

        let disconnect_player_ids = connection_handler.get_disconnected_player_ids();

        let disconnect_event = EventOut::disconnect_event(disconnect_player_ids);

        connection_handler.clean_disconnected_list();

        let players = game_state.all_players();
        for player in players {
            match player.connection_status {
                ConnectionStatus::Connected => {
                    let maybe_identifier = connection_handler.get_connected_identifier(&player.id);

                    if let Some(identifier) = maybe_identifier {
                        if let Some(ref pe) = position_event {
                            let packet = pe.get_with_event_header(identifier.clone());
                            tracing::info!("Sent: {:?}", pe);
                            socket
                                .lock()
                                .await
                                .send_to(packet.as_slice(), player.addr)
                                .await
                                .unwrap();
                        }
                        if let Some(ref se) = spawn_event {
                            tracing::info!("Sent : {:?}", se);
                            let packet = se.get_with_event_header(identifier.clone());
                            socket
                                .lock()
                                .await
                                .send_to(packet.as_slice(), player.addr)
                                .await
                                .unwrap();
                        }
                        if let Some(ref de) = disconnect_event {
                            tracing::info!("Sent : {:?}", de);
                            let packet = de.get_with_event_header(identifier.clone());
                            socket
                                .lock()
                                .await
                                .send_to(packet.as_slice(), player.addr)
                                .await
                                .unwrap();
                        }
                    }
                }
                ConnectionStatus::Connecting => {
                    let maybe_identifier = connection_handler.get_pending_identifier(&player.id);
                    if let Some(identifier) = maybe_identifier {
                        let spawn_event_connected_player =
                            EventOut::spawn_event(connected_players.clone());

                        if let Some(con_se) = spawn_event_connected_player {
                            tracing::info!("Sent : {:?}", con_se);
                            let packet = con_se.get_with_event_header(identifier.clone());
                            socket
                                .lock()
                                .await
                                .send_to(packet.as_slice(), player.addr)
                                .await
                                .unwrap();
                        }

                        if let Some(ref se) = spawn_event {
                            tracing::info!("Sent : {:?}", se);
                            let packet = se.get_with_event_header(identifier.clone());
                            socket
                                .lock()
                                .await
                                .send_to(packet.as_slice(), player.addr)
                                .await
                                .unwrap();
                        }
                        connection_handler.set_connected(&player.id);
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
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
    }
}

pub struct ServerOptions {
    tick_rate: f32,
}
