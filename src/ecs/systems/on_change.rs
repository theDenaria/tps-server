use bevy_ecs::{
    event::EventReader,
    query::Changed,
    schedule::SystemSet,
    system::{Query, ResMut},
};
use rapier3d::math::{Real, Vector};

use crate::{
    ecs::{
        components::{Health, Player},
        events::{PositionChangeEvent, RotationChangeEvent},
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
        positions.push((event.translation, event.player_id.clone()));
    }

    if let Some(position_event) = MessageOut::position_message(positions) {
        server.broadcast_message(DefaultChannel::Unreliable, position_event.data);
    }
}

pub fn on_rotation_change(
    mut rotation_change_events: EventReader<RotationChangeEvent>,
    mut server: ResMut<MattaServer>,
) {
    let mut rotations: Vec<(Vector<Real>, String)> = vec![];
    for event in rotation_change_events.read() {
        rotations.push((event.rotation, event.player_id.clone()));
    }

    if let Some(rotation_event) = MessageOut::rotation_message(rotations) {
        server.broadcast_message(DefaultChannel::Unreliable, rotation_event.data);
    }
}

pub fn on_health_change(query: Query<(&Player, &Health), Changed<Health>>) {
    for (player, health) in &query {
        // Broadcast health update
    }
}
