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

use tracing_subscriber::{EnvFilter, FmtSubscriber};

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use crate::{
    channel::DefaultChannel,
    connection::ConnectionConfig,
    event_in::{EventIn, EventInType},
    event_out::EventOut,
    server::{ClientId, MattaServer, ServerEvent},
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
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    println!("Client {client_id} disconnected: {reason}");
                }
            }
        }

        // Receive message from channel
        for client_id in server.clients_id() {
            // The enum DefaultChannel describe the channels used by the default configuration
            while let Some(message) =
                server.receive_message(client_id, DefaultChannel::ReliableOrdered)
            {
                let event_in = EventIn::new(message.to_vec()).unwrap();

                match event_in.event_type {
                    EventInType::Connect => {
                        tracing::trace!("Connect EVENT Received!");
                        let event_out = EventOut::spawn_event_by_player_id(event_in.player_id);
                        server.broadcast_message(DefaultChannel::ReliableOrdered, event_out.data);
                        tracing::trace!("Spawn event broadcasted!");
                    }
                    _ => {}
                }

                tracing::info!("Received message bytes: {:?}", message);
            }
        }

        // Send a text message for all clients
        server.broadcast_message(DefaultChannel::ReliableOrdered, "server message");

        let client_id = ClientId::from_raw(0);
        // Send a text message for all clients except for Client 0
        server.broadcast_message_except(
            client_id,
            DefaultChannel::ReliableOrdered,
            "server message",
        );

        // Send message to only one client
        // server.send_message(client_id, DefaultChannel::ReliableOrdered, "server message");

        // Send packets to clients using the transport layer
        transport.send_packets(&mut server);

        std::thread::sleep(delta_time); // Running at 60hz
    }
}
