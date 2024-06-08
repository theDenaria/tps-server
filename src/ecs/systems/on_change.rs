use bevy_ecs::{
    query::Changed,
    schedule::SystemSet,
    system::{Query, ResMut},
};
use rapier3d::na::{UnitQuaternion, Vector3};

use crate::{
    ecs::components::{Health, IsGrounded, Player, Position, Rotation, VerticalVelocity},
    server::{channel::DefaultChannel, message_out::MessageOut, server::MattaServer},
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HandleGameStateChanges;

// Gets the Position component of all Entities whose Velocity has changed since the last run of the System
pub fn on_position_change(
    query: Query<(&Player, &Position), Changed<Position>>,
    mut server: ResMut<MattaServer>,
) {
    let mut positions: Vec<(Vector3<f32>, String)> = vec![];

    for (player, position) in &query {
        positions.push((position.0, player.id.clone()));
    }

    if let Some(position_event) = MessageOut::position_message(positions) {
        server.broadcast_message(DefaultChannel::Unreliable, position_event.data);
    }
}

pub fn on_rotation_change(
    query: Query<(&Player, &Rotation), Changed<Rotation>>,
    mut server: ResMut<MattaServer>,
) {
    let mut rotations: Vec<(UnitQuaternion<f32>, String)> = vec![];
    for (player, rotation) in &query {
        rotations.push((rotation.0, player.id.clone()));
    }

    if let Some(rotation_message) = MessageOut::rotation_message(rotations) {
        server.broadcast_message(DefaultChannel::Unreliable, rotation_message.data);
    }
}

pub fn on_health_change(
    query: Query<(&Player, &Health), Changed<Health>>,
    mut server: ResMut<MattaServer>,
) {
    let mut healths: Vec<(String, f32)> = vec![];
    for (player, health) in &query {
        healths.push((player.id.clone(), health.0));
    }
    if healths.len() > 0 {
        let health_message = MessageOut::health_message(healths);
        server.broadcast_message(DefaultChannel::ReliableOrdered, health_message.data);
    }
}

pub fn on_grounded_change(
    mut query: Query<(&mut VerticalVelocity, &IsGrounded), Changed<IsGrounded>>,
) {
    for (mut v_velocity, is_grounded) in &mut query {
        if is_grounded.0 {
            v_velocity.0 = 0.0;
        }
    }
}
