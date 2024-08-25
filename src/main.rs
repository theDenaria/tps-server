#![allow(dead_code)]
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    time::SystemTime,
};
mod constants;
mod ecs;
mod server;
mod sessions;

use bevy::prelude::*;

use server::{
    channel::DefaultChannel,
    connection::ConnectionConfig,
    message_in::{MessageIn, MessageInType},
    server::DenariaServer,
    transport::{server::server::ServerConfig, transport::ServerTransport},
};
use sessions::{NetworkResource, SessionHandler};

// #[tokio::main]
fn main() -> io::Result<()> {
    let mut server = Arc::new(Mutex::new(DenariaServer::new(ConnectionConfig::default())));
    // Setup transport layer
    const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
    let socket: UdpSocket = UdpSocket::bind(SERVER_ADDR)?;
    let server_config = ServerConfig {
        current_time: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap(),
        max_clients: 64,
        public_addresses: vec![SERVER_ADDR],
    };
    let transport = Arc::new(Mutex::new(ServerTransport::new(server_config, socket)?));

    let mut session_handler = Arc::new(Mutex::new(SessionHandler::new()));
    let mut session_handler_lock = session_handler.lock().unwrap();
    loop {
        server
            .lock()
            .unwrap()
            .clients_id()
            .iter()
            .for_each(|client_id| {
                while let Some((message, player_id)) = server
                    .lock()
                    .unwrap()
                    .receive_message(*client_id, DefaultChannel::Unreliable)
                {
                    let event_in = MessageIn::new(player_id.clone(), message.to_vec()).unwrap();

                    match event_in.event_type {
                        MessageInType::Rotation => match event_in.to_look_event() {
                            Ok(event) => {
                                session_handler_lock.send_event(player_id, event);
                            }
                            Err(_) => {}
                        },
                        MessageInType::Move => match event_in.to_move_event() {
                            Ok(event) => {
                                session_handler_lock.send_event(player_id, event);
                            }
                            Err(_) => {}
                        },
                        MessageInType::Fire => match event_in.to_fire_event() {
                            Ok(event) => {
                                session_handler_lock.send_event(player_id, event);
                            }
                            Err(_) => {}
                        },
                        MessageInType::Jump => match event_in.to_jump_event() {
                            Ok(event) => {
                                session_handler_lock.send_event(player_id, event);
                            }
                            Err(_) => {}
                        },

                        MessageInType::Connect => match event_in.to_connect_event() {
                            Ok(event) => {
                                session_handler_lock.send_event(player_id, event);
                            }
                            Err(_) => {}
                        },
                        MessageInType::SessionCreate => match event_in.to_session_create_input() {
                            Ok(input) => {
                                let network_resource = NetworkResource {
                                    server: server.clone(),
                                    transport: transport.clone(),
                                };
                                let session_handler_clone = session_handler.clone();
                                let handle = std::thread::spawn(|| {
                                    session_handler_clone
                                        .lock()
                                        .unwrap()
                                        .new_session(input, network_resource);
                                });
                            }
                            Err(_) => {}
                        },
                        MessageInType::Invalid => {
                            error!("Invalid MessageInType");
                        }
                    }
                }
            });
    }

    Ok(())
}
