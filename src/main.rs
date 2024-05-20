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

use game_state::{send_connect_event, ConnectEvent, JumpEvent, LookEvent, MoveEvent, PlayerLookup};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use crate::{
    channel::DefaultChannel,
    connection::ConnectionConfig,
    event_in::{digest_move_event, digest_rotation_event, EventIn, EventInType},
    game_state::{
        handle_connect_events, handle_disconnect_events, handle_fire_events, handle_look_events,
        handle_move_events, on_health_change, on_player_added, on_position_change,
        on_rotation_change, send_disconnect_event, send_look_event, send_move_event,
        DisconnectEvent, FireEvent, HandleGameEvents, HandleGameStateChanges,
    },
    server::{MattaServer, ServerEvent},
    transport::{error::TransportError, server::server::ServerConfig, transport::ServerTransport},
};

use bevy_ecs::prelude::*;

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
    let server = MattaServer::new(ConnectionConfig::default());

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
    let transport = ServerTransport::new(server_config, socket).unwrap();

    let mut world = World::default();

    let duration = DurationResource::default();

    world.insert_resource(server);
    world.insert_resource(transport);
    world.insert_resource(PlayerLookup::new());
    world.insert_resource(duration);

    // Register events
    world.insert_resource(Events::<ConnectEvent>::default());
    world.insert_resource(Events::<DisconnectEvent>::default());
    world.insert_resource(Events::<MoveEvent>::default());
    world.insert_resource(Events::<LookEvent>::default());
    world.insert_resource(Events::<JumpEvent>::default());
    world.insert_resource(Events::<FireEvent>::default());

    let mut schedule = Schedule::default();

    schedule.add_systems((
        receive_server_events.in_set(RecServerEvents),
        handle_server_events
            .in_set(HandleServerEvents)
            .after(RecServerEvents),
        (
            handle_move_events,
            handle_look_events,
            handle_fire_events,
            handle_connect_events,
            handle_disconnect_events,
        )
            .in_set(HandleGameEvents)
            .after(HandleServerEvents),
        (
            on_player_added,
            on_position_change,
            on_rotation_change,
            on_health_change,
        )
            .in_set(HandleGameStateChanges)
            .after(HandleGameEvents),
        transport_send_packets.after(HandleGameStateChanges),
    ));

    // Your gameplay loop
    loop {
        // Receive new messages and update clients
        schedule.run(&mut world);
    }
}

// Define system labels for ordering
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct RecServerEvents;

fn receive_server_events(
    mut server: ResMut<MattaServer>,
    mut transport: ResMut<ServerTransport>,
    mut delta_time: ResMut<DurationResource>,
    mut disconnect_event: EventWriter<DisconnectEvent>,
) {
    delta_time.0 = Duration::from_millis(16);
    server.update(delta_time.0);
    transport.update(delta_time.0, &mut server).unwrap();

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
                send_disconnect_event(player_id, &mut disconnect_event);
            }
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct HandleServerEvents;

fn handle_server_events(
    mut server: ResMut<MattaServer>,
    player_lookup: Res<PlayerLookup>,
    mut connect_event: EventWriter<ConnectEvent>,
    mut move_event: EventWriter<MoveEvent>,
    mut look_event: EventWriter<LookEvent>,
    // mut jump_event: EventWriter<JumpEvent>,
) {
    // Receive message from channel

    server.clients_id().iter().for_each(|client_id| {
        while let Some((message, player_id)) =
            server.receive_message(*client_id, DefaultChannel::Unreliable)
        {
            let event_in = EventIn::new(message.to_vec()).unwrap();

            match event_in.event_type {
                EventInType::Rotation => {
                    let rotation = digest_rotation_event(event_in.data).unwrap();
                    send_look_event(
                        player_id,
                        0.0,
                        rotation,
                        0.0,
                        &player_lookup,
                        &mut look_event,
                    );
                    tracing::trace!("Rotation Message Received: {:?}", rotation);
                }
                EventInType::Move => {
                    let move_event_in = digest_move_event(event_in.data).unwrap();
                    send_move_event(
                        player_id,
                        move_event_in.x,
                        move_event_in.y,
                        &player_lookup,
                        &mut move_event,
                    );
                    tracing::trace!("Move Message Received: {:?}", move_event_in);
                }
                EventInType::Connect => {
                    send_connect_event(player_id, &mut connect_event);
                }
                EventInType::Invalid => {
                    tracing::error!("Invalied EventInType");
                }
            }
        }
    });
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct SendTransportPackages;

fn transport_send_packets(
    mut server: ResMut<MattaServer>,
    mut transport: ResMut<ServerTransport>,
    delta_time: Res<DurationResource>,
) {
    transport.send_packets(&mut server);
    std::thread::sleep(delta_time.0); // Running at 60hz
}

struct DurationResource(Duration);

impl Default for DurationResource {
    fn default() -> Self {
        DurationResource(Duration::default())
    }
}

impl Resource for DurationResource {}
