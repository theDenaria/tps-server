use bevy::prelude::{EventWriter, ResMut};

use crate::{
    constants::TICK_DELTA,
    ecs::{events::DisconnectEvent, systems::send_events::send_disconnect_event},
    server::{
        server::{DenariaServer, ServerEvent},
        transport::transport::ServerTransport,
    },
};

pub fn handle_server_events(
    mut server: ResMut<DenariaServer>,
    mut transport: ResMut<ServerTransport>,
    mut disconnect_event: EventWriter<DisconnectEvent>,
) {
    server.update(TICK_DELTA);
    transport.update(TICK_DELTA, &mut server).unwrap();

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

pub fn transport_send_packets(
    mut server: ResMut<DenariaServer>,
    mut transport: ResMut<ServerTransport>,
) {
    transport.send_packets(&mut server);
}
