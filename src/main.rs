#![allow(dead_code)]
use std::io;
mod channel;
mod connection;
mod connection_stats;
mod constants;
mod error;
mod event_in;
mod event_out;
mod game_state;
mod packet;
mod server;
mod transport;

use bytes::Bytes;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use crate::{
    channel::DefaultChannel,
    connection::ConnectionConfig,
    event_in::{digest_move_event, digest_rotation_event, EventIn, EventInType},
    event_out::EventOut,
    game_state::GameState,
    packet::Packet,
    server::{MattaServer, ServerEvent},
    transport::{error::TransportError, server::server::ServerConfig, transport::ServerTransport},
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Now the tracing macros can be used throughout your application
    tracing::info!("This will dynamically update on the terminal");

    let _ = start_server();

    Ok(())
}

fn start_server() -> Result<(), TransportError> {
    let mut server = MattaServer::new(ConnectionConfig::default());

    // Setup transport layer
    const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
    let socket: UdpSocket = UdpSocket::bind(SERVER_ADDR).unwrap();
    let server_config = ServerConfig {
        current_time: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap(),
        max_clients: 64,
        public_addresses: vec![SERVER_ADDR],
    };
    let mut transport = ServerTransport::new(server_config, socket).unwrap();

    let mut game_state = GameState::new();

    // Your gameplay loop
    loop {
        let delta_time = Duration::from_millis(16);
        // Receive new messages and update clients
        server.update(delta_time);
        transport.update(delta_time, &mut server)?;

        // Check for client connections/disconnections
        while let Some(event) = server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    println!("Client {client_id} connected");
                }
                ServerEvent::ClientDisconnected {
                    client_id,
                    player_id,
                    reason,
                } => {
                    println!("Client {client_id} disconnected: {reason}");
                    game_state.remove_player(&player_id);
                    let disconnect_event = EventOut::disconnect_event(vec![&player_id]).unwrap();
                    tracing::trace!("Disconnect event: {:?}", disconnect_event);
                    server
                        .broadcast_message(DefaultChannel::ReliableOrdered, disconnect_event.data);
                }
            }
        }

        // Receive message from channel
        for client_id in server.clients_id() {
            // The enum DefaultChannel describe the channels used by the default configuration
            while let Some((message, player_id)) =
                server.receive_message(client_id, DefaultChannel::Unreliable)
            {
                let event_in = EventIn::new(message.to_vec()).unwrap();

                match event_in.event_type {
                    EventInType::Connect => {
                        let player_id = player_id.clone();
                        match game_state.get_player(&player_id) {
                            None => {
                                let spawned_players = game_state.all_players();
                                if let Some(spawn_players) = EventOut::spawn_event(spawned_players)
                                {
                                    server.send_message(
                                        client_id,
                                        DefaultChannel::ReliableOrdered,
                                        spawn_players.data,
                                    );
                                }

                                game_state.add_player(&player_id);
                                let spawn_new_player =
                                    EventOut::spawn_event_by_player_id(&player_id);
                                server.broadcast_message(
                                    DefaultChannel::ReliableOrdered,
                                    spawn_new_player.data,
                                );
                            }
                            Some(_) => {}
                        }
                    }
                    EventInType::Rotation => {
                        let rotation = digest_rotation_event(event_in.data).unwrap();
                        if let Some(player) = game_state.get_player_mut(player_id) {
                            player.update_rotation(rotation);
                            let position_event = EventOut::position_event(vec![player]).unwrap();
                            server
                                .broadcast_message(DefaultChannel::Unreliable, position_event.data);
                        }

                        tracing::trace!("Rotation Message Received: {:?}", rotation);
                    }
                    EventInType::Move => {
                        let move_event = digest_move_event(event_in.data).unwrap();
                        tracing::trace!("Rotation Message Received: {:?}", move_event);
                        if let Some(player) = game_state.get_player_mut(player_id) {
                            player.update_position(move_event);
                            let position_event = EventOut::position_event(vec![player]).unwrap();
                            server
                                .broadcast_message(DefaultChannel::Unreliable, position_event.data);
                        }
                    }
                    EventInType::Invalid => {
                        tracing::error!("Invalied EventInType");
                    }
                }
            }
        }
        // Send packets to clients using the transport layer
        transport.send_packets(&mut server);

        std::thread::sleep(delta_time); // Running at 60hz
    }
}

fn run_tests() {
    let event_out = EventOut::spawn_event_by_player_id(&String::from("player1"));

    tracing::trace!("Event Data : {:?}", event_out);

    let messages = vec![Bytes::from(event_out.data)];

    let sample = Packet::SmallUnreliable {
        channel_id: 0,
        messages: messages.clone(),
    };

    let sample_reliable = Packet::SmallReliable {
        channel_id: 1,
        packet_type: 0,
        packet_process_time: 0,
        sequence_id: 1,
        acked_seq_id: u16::MAX,
        acked_mask: 0,
        messages: vec![(0, messages[0].clone()), (1, messages[0].clone())],
    };

    let mut buffer = [0u8; 1400];

    let len = sample_reliable.to_bytes(&mut buffer).unwrap();

    tracing::trace!("SAMPLE PACKET: {:?}", sample_reliable);
    tracing::trace!("SAMPLE PACKET to bytes: {:?}", &buffer[..len]);

    let len = sample.to_bytes(&mut buffer).unwrap();

    tracing::trace!("SAMPLE PACKET: {:?}", sample);
    tracing::trace!("SAMPLE PACKET to bytes: {:?}", &buffer[..len]);
}
