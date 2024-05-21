use bevy_ecs::{
    event::EventReader,
    query::{Added, Changed},
    schedule::SystemSet,
    system::{Query, ResMut},
};
use rapier3d::math::{Real, Vector};

use crate::{
    ecs::{
        components::{Health, Player},
        events::PositionChangeEvent,
    },
    server::{channel::DefaultChannel, message_out::MessageOut, server::MattaServer},
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HandleGameStateChanges;

// Gets the Position component of all Entities whose Velocity has changed since the last run of the System
pub fn on_position_change(
    mut position_change_events: EventReader<PositionChangeEvent>,
    mut server: ResMut<MattaServer>,
) {
    let mut positions: Vec<(Vector<Real>, String)> = vec![];
    for event in position_change_events.read() {
        positions.push((event.translation, event.player_id));
    }

    if let Some(position_event) = MessageOut::position_message(positions) {
        server.broadcast_message(DefaultChannel::Unreliable, position_event.data);
    }
}

pub fn on_rotation_change(
    query: Query<(&Player, &Rotation), Changed<Rotation>>,
    mut server: ResMut<MattaServer>,
) {
    if let Some(rotation_event) = MessageOut::rotation_message(&query) {
        tracing::trace!("ROTATION EVENT TO SEND: {:?}", rotation_event);
        server.broadcast_message(DefaultChannel::Unreliable, rotation_event.data);
    }
}

pub fn on_health_change(query: Query<(&Player, &Health), Changed<Health>>) {
    for (player, health) in &query {
        // Broadcast health update
    }
}

pub fn on_player_added(
    added_players_query: Query<(&Player, &Position, &Rotation, &Health), Added<Player>>,
    all_players_query: Query<(&Player, &Position, &Rotation)>,
    mut server: ResMut<MattaServer>,
) {
    for (player, position, rotation, health) in added_players_query.iter() {
        if let Ok(added_client_id) = server.client_id_by_player_id(player.id.clone()) {
            if let Some(spawn_all) = MessageOut::spawn_message_for_all_players(&all_players_query) {
                server.send_message(
                    added_client_id,
                    DefaultChannel::ReliableOrdered,
                    spawn_all.data,
                )
            }
            let spawn = MessageOut::spawn_new_message(
                player.id.clone(),
                position.clone(),
                rotation.clone(),
            );
            server.broadcast_message_except(
                added_client_id,
                DefaultChannel::ReliableOrdered,
                spawn.data,
            )
        }
    }
}
