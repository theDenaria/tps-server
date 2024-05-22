use bevy_ecs::{
    event::{EventReader, EventWriter},
    schedule::SystemSet,
    system::{Commands, Query, ResMut},
};

use rapier3d::{na::UnitQuaternion, prelude::*};

use crate::{
    constants::VELOCITY_MUL,
    ecs::{
        components::{Health, Player, PlayerBundle, PlayerLookup, PlayerPhysics},
        events::{
            ConnectEvent, DisconnectEvent, FireEvent, LookEvent, MoveEvent, PositionChangeEvent,
            RotationChangeEvent,
        },
    },
    server::{channel::DefaultChannel, message_out::MessageOut, server::MattaServer},
};

use super::physics::PhysicsResources;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HandleGameEvents;

pub fn handle_move_events(
    mut move_events: EventReader<MoveEvent>,
    query: Query<(&Player, &PlayerPhysics)>,
    mut physics_res: ResMut<PhysicsResources>,
    mut position_update_event: EventWriter<PositionChangeEvent>,
) {
    for event in move_events.read() {
        if let Ok((player, physics)) = query.get(event.entity) {
            let PhysicsResources {
                rigid_body_set,
                collider_set,
                query_pipeline,
                character_controller,
                ..
            } = &mut *physics_res;

            let rigid_body_handle = physics.rigid_body_handle;
            let collider_handle = physics.collider_handle;

            let current_translation = rigid_body_set[rigid_body_handle].translation();
            let current_position = rigid_body_set[rigid_body_handle].position();
            let collider_shape = collider_set[collider_handle].shape().clone();

            let direction = vector![event.x, 0.0, event.y];
            let normalized_direction = if direction.magnitude() > 0.0 {
                direction.normalize()
            } else {
                direction
            };

            let dt = 1.0 / 60.0; // Example timestep (assuming 60 FPS)

            let desired_translation = normalized_direction * VELOCITY_MUL * dt;

            // Calculate the target position by adding the desired translation
            let desired_translation = current_translation + desired_translation;

            let corrected_movement = character_controller.move_shape(
                dt,               // The timestep length (can be set to SimulationSettings::dt).
                rigid_body_set,   // The RigidBodySet.
                collider_set,     // The ColliderSet.
                query_pipeline,   // The QueryPipeline.
                collider_shape,   // The character’s shape.
                current_position, // The character’s initial position.
                desired_translation,
                QueryFilter::default()
                    // Make sure the character we are trying to move isn’t considered an obstacle.
                    .exclude_rigid_body(physics.rigid_body_handle),
                |_| {}, // We don’t care about events in this example.
            );

            let rigid_body = rigid_body_set.get_mut(physics.rigid_body_handle).unwrap();
            rigid_body.set_translation(corrected_movement.translation, true);

            tracing::info!("Position update event sent");
            position_update_event.send(PositionChangeEvent {
                player_id: player.id.clone(),
                translation: corrected_movement.translation,
            });
        }
    }
}

pub fn handle_look_events(
    mut look_events: EventReader<LookEvent>,
    query: Query<(&Player, &PlayerPhysics)>,
    mut physics_res: ResMut<PhysicsResources>,
    mut rotation_update_event: EventWriter<RotationChangeEvent>,
) {
    for event in look_events.read() {
        if let Ok((player, physics)) = query.get(event.entity) {
            let PhysicsResources { rigid_body_set, .. } = &mut *physics_res;
            let rigid_body = rigid_body_set.get_mut(physics.rigid_body_handle).unwrap();
            let rotation = vector![event.x, event.y, event.z];
            let rot_quaternion = UnitQuaternion::from_euler_angles(event.x, event.y, event.z);
            rigid_body.set_rotation(rot_quaternion, true);

            rotation_update_event.send(RotationChangeEvent {
                player_id: player.id.clone(),
                rotation,
            });
        }
    }
}

pub fn handle_jump_events(mut jump_events: EventReader<LookEvent>) {
    for event in jump_events.read() {}
}

pub fn handle_fire_events(mut fire_events: EventReader<FireEvent>, mut query: Query<&mut Health>) {
    for event in fire_events.read() {}
}

pub fn handle_connect_events(
    mut commands: Commands,
    mut connect_events: EventReader<ConnectEvent>,
    mut player_lookup: ResMut<PlayerLookup>,
    mut physics_res: ResMut<PhysicsResources>,
    mut position_change_event: EventWriter<PositionChangeEvent>,
) {
    for event in connect_events.read() {
        if !player_lookup.map.contains_key(&event.player_id) {
            let initial_translation = vector![0.0, 5.0, 1.0];
            let rigid_body = RigidBodyBuilder::new(RigidBodyType::KinematicPositionBased)
                // The rigid body translation.
                // Default: zero vector.
                .translation(initial_translation.clone())
                // All done, actually build the rigid-body.
                .build();
            let collider = ColliderBuilder::capsule_y(0.5, 0.2).build();

            let PhysicsResources {
                rigid_body_set,
                collider_set,
                ..
            } = &mut *physics_res;

            let rigid_body_handle = rigid_body_set.insert(rigid_body);

            let collider_handle =
                collider_set.insert_with_parent(collider, rigid_body_handle, rigid_body_set);

            let entity = commands
                .spawn({
                    PlayerBundle {
                        player: Player {
                            id: event.player_id.clone(),
                        },
                        physics: PlayerPhysics {
                            rigid_body_handle,
                            collider_handle,
                        },
                        ..Default::default()
                    }
                })
                .id();
            player_lookup.map.insert(event.player_id.clone(), entity);

            position_change_event.send(PositionChangeEvent {
                player_id: event.player_id.clone(),
                translation: initial_translation,
            });
        }
    }
}

pub fn handle_disconnect_events(
    mut commands: Commands,
    mut disconnect_events: EventReader<DisconnectEvent>,
    mut player_lookup: ResMut<PlayerLookup>,
    mut server: ResMut<MattaServer>,
) {
    if disconnect_events.len() > 0 {
        let mut disconnect_player_ids: Vec<&String> = vec![];
        for event in disconnect_events.read() {
            if let Some(entity) = player_lookup.map.get(&event.player_id) {
                commands.entity(*entity).despawn();
                player_lookup.map.remove(&event.player_id);

                disconnect_player_ids.push(&event.player_id);
            }
        }
        let disconnect_event = MessageOut::disconnect_message(disconnect_player_ids).unwrap();
        tracing::trace!("Disconnect event: {:?}", disconnect_event);
        server.broadcast_message(DefaultChannel::ReliableOrdered, disconnect_event.data);
    }
}
