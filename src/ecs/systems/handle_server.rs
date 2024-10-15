use bevy::prelude::{EventWriter, Query, Res, ResMut};

use crate::{
    constants::TICK_DELTA,
    ecs::{
        components::{MoveInput, PlayerLookup},
        events::{DisconnectEvent, FireEvent, LookEvent, SpawnEvent},
    },
    server::{
        channel::DefaultChannel,
        message_in::{MessageIn, MessageInType},
        server::{DenariaServer, ServerEvent},
    },
};

pub fn handle_server_events(
    mut server: ResMut<DenariaServer>,
    mut disconnect_event: EventWriter<DisconnectEvent>,
) {
    server.update(TICK_DELTA);
    server.process_server_transport_messages();

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
                disconnect_event.send(DisconnectEvent { player_id });
            }
        }
    }
}

pub fn handle_server_messages(
    mut server: ResMut<DenariaServer>,
    player_lookup: Res<PlayerLookup>,
    mut spawn_event: EventWriter<SpawnEvent>,
    mut move_query: Query<&mut MoveInput>,
    mut look_event: EventWriter<LookEvent>,
    mut fire_event: EventWriter<FireEvent>,
) {
    // Receive message from channel

    server.clients_id().iter().for_each(|client_id| {
        while let Some((message, player_id)) =
            server.receive_message(*client_id, DefaultChannel::Unreliable)
        {
            let event_in = match MessageIn::new(message.to_vec(), player_id.clone()) {
                Ok(event) => event,
                Err(e) => {
                    tracing::error!("Failed to create MessageIn: {}", e);
                    continue;
                }
            };

            match event_in.event_type {
                MessageInType::Rotation => {
                    if let Some(player_entity) = player_lookup.map.get(player_id) {
                        match event_in.to_look_event(*player_entity) {
                            Ok(event) => {
                                look_event.send(event);
                            }
                            Err(_) => {
                                tracing::error!("Failed to create LookEvent");
                            }
                        }
                    }
                }
                MessageInType::Move => {
                    if let Some(player_entity) = player_lookup.map.get(player_id) {
                        match event_in.to_move_event(*player_entity) {
                            Ok(event) => {
                                if let Ok(mut move_entity) = move_query.get_mut(event.entity) {
                                    move_entity.x = event.x;
                                    move_entity.z = event.y;
                                }
                            }
                            Err(_) => {
                                tracing::error!("Failed to create MoveEvent");
                            }
                        }
                    }
                }
                MessageInType::Fire => {
                    if let Some(player_entity) = player_lookup.map.get(player_id) {
                        match event_in.to_fire_event(*player_entity) {
                            Ok(event) => {
                                fire_event.send(event);
                            }
                            Err(_) => {
                                tracing::error!("Failed to create FireEvent");
                            }
                        }
                    }
                }
                MessageInType::Jump => {
                    if let Some(player_entity) = player_lookup.map.get(player_id) {
                        match event_in.to_jump_event(*player_entity) {
                            Ok(event) => {
                                if let Ok(mut move_entity) = move_query.get_mut(event.entity) {
                                    move_entity.y = 1.0;
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
                MessageInType::Spawn => match event_in.to_spawn_event() {
                    Ok(event) => {
                        tracing::info!("Sending spawn event to session");
                        spawn_event.send(event);
                        tracing::info!("Sent spawn event to session");
                    }
                    Err(_) => {}
                },
                MessageInType::Invalid => {
                    tracing::error!("Invalid MessageInType");
                }
            }
        }
    });
}

pub fn handle_outgoing_messages(mut server: ResMut<DenariaServer>) {
    for client_id in server.clients_id() {
        let packets = server.get_packets_to_send(client_id).unwrap();
        server.send_packets_to_server_transport(client_id, packets);
    }
}
