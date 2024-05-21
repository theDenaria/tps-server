use std::time::Duration;

use bevy_ecs::{
    event::EventWriter,
    schedule::SystemSet,
    system::{Res, ResMut},
};

use crate::{
    ecs::{
        components::PlayerLookup,
        events::{ConnectEvent, DisconnectEvent, LookEvent, MoveEvent},
        systems::send_events::{send_disconnect_event, send_look_event, send_move_event},
    },
    server::{
        channel::DefaultChannel,
        message_in::{digest_move_message, digest_rotation_message, MessageIn, MessageInType},
        server::{MattaServer, ServerEvent},
        transport::transport::ServerTransport,
    },
};

use super::{send_events::send_connect_event, setup::DurationResource};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HandleServer;

pub fn handle_server_events(
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

pub fn handle_server_messages(
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
            let event_in = MessageIn::new(message.to_vec()).unwrap();

            match event_in.event_type {
                MessageInType::Rotation => {
                    let rotation = digest_rotation_message(event_in.data).unwrap();
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
                MessageInType::Move => {
                    let move_event_in = digest_move_message(event_in.data).unwrap();
                    send_move_event(
                        player_id,
                        move_event_in.x,
                        move_event_in.y,
                        &player_lookup,
                        &mut move_event,
                    );
                    tracing::trace!("Move Message Received: {:?}", move_event_in);
                }
                MessageInType::Connect => {
                    send_connect_event(player_id, &mut connect_event);
                }
                MessageInType::Invalid => {
                    tracing::error!("Invalied MessageInType");
                }
            }
        }
    });
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct SendPackets;

pub fn transport_send_packets(
    mut server: ResMut<MattaServer>,
    mut transport: ResMut<ServerTransport>,
    delta_time: Res<DurationResource>,
) {
    transport.send_packets(&mut server);
    std::thread::sleep(delta_time.0); // Running at 60hz
}
