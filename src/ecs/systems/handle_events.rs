use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::{prelude::*, rapier::prelude::RigidBodySet};

use crate::{
    constants::{GRAVITY, JUMP_SPEED, VELOCITY_MUL},
    ecs::{
        components::{Health, MoveInput, Player, PlayerBundle, PlayerLookup, VerticalVelocity},
        events::{ConnectEvent, DisconnectEvent, FireEvent, HitEvent, LookEvent},
    },
    server::{channel::DefaultChannel, message_out::MessageOut, server::DenariaServer},
};

pub fn handle_character_movement(
    time: Res<Time>,
    mut query: Query<(
        &mut KinematicCharacterController,
        &mut MoveInput,
        &mut VerticalVelocity,
        Option<&KinematicCharacterControllerOutput>,
    )>,
) {
    let delta_time = time.delta_seconds();
    for (mut controller, mut move_input, mut v_velocity, output) in query.iter_mut() {
        let mut movement = Vec3::new(move_input.x, 0.0, move_input.z) * VELOCITY_MUL;

        if output.map(|o| o.grounded).unwrap_or(false) {
            v_velocity.0 = move_input.y * JUMP_SPEED;
        } else {
            v_velocity.0 -= GRAVITY * delta_time * controller.custom_mass.unwrap_or(1.0);
        }

        move_input.x = 0.0;
        move_input.y = 0.0;
        move_input.z = 0.0;

        movement.y = v_velocity.0;
        controller.translation = Some(movement);
    }
}

pub fn handle_look_events(
    mut look_events: EventReader<LookEvent>,
    mut query: Query<&mut Transform>,
) {
    for event in look_events.read() {
        if let Ok(mut transform) = query.get_mut(event.entity) {
            transform.rotation = Quat::from_vec4(event.direction);
        }
    }
}

// TODO: Fire angle calculations needs to be fixed
pub fn handle_fire_events(
    mut fire_events: EventReader<FireEvent>,
    query: Query<&Player>,
    rapier_context: Res<RapierContext>,
    mut hit_event: EventWriter<HitEvent>,
    mut server: ResMut<DenariaServer>,
) {
    let max_toi = f32::MAX;
    let solid = true;

    for event in fire_events.read() {
        if let Ok(player) = query.get(event.entity) {
            if let Some((initial_handle, initial_toi)) = rapier_context.cast_ray(
                event.cam_origin,
                event.direction,
                max_toi,
                solid,
                QueryFilter::default().exclude_collider(event.entity),
            ) {
                let initial_hit_point = event.cam_origin * event.direction * initial_toi;

                // Second raycast from the barrel position to the initial hit point
                let barrel_target_dir = initial_hit_point - event.barrel_origin;

                let normalized_a = event.direction.normalize();
                let normalized_b = barrel_target_dir.normalize();

                // Compute the dot product
                let dot_product = normalized_a.dot(normalized_b);

                // Clamp the dot product to the valid range for acos ([-1.0, 1.0])
                let clamped_dot = dot_product.clamp(-1.0, 1.0);

                // Compute the angle in radians
                let angle_in_radians = clamped_dot.acos();

                // Optionally, convert to degrees if needed
                let angle_in_degrees = angle_in_radians * 180.0 / PI;

                let angle_threshold = 70.0;

                tracing::info!(
                    "Angle: {:?}, Threshold: {:?}",
                    angle_in_degrees,
                    angle_threshold
                );

                if angle_in_degrees <= angle_threshold {
                    if let Some((handle, toi)) = rapier_context.cast_ray(
                        event.barrel_origin,
                        barrel_target_dir,
                        max_toi,
                        solid,
                        QueryFilter::default().exclude_collider(event.entity),
                    ) {
                        let hit_point = event.barrel_origin * barrel_target_dir * toi;
                        tracing::info!("Main target or an obstacle hit");

                        hit_event.send(HitEvent {
                            hitter_id: player.id.clone(),
                            hitten: handle,
                            weapon: String::from("pistol"),
                            point: hit_point,
                        });

                        let fire_message = MessageOut::fire_message(
                            player.id.clone(),
                            event.barrel_origin,
                            barrel_target_dir,
                        );
                        server
                            .broadcast_message(DefaultChannel::ReliableOrdered, fire_message.data);
                    }
                } else {
                    // No obstacle between the barrel and the target, so use the initial hit point

                    tracing::info!("Main target threshold misses");
                    hit_event.send(HitEvent {
                        hitter_id: player.id.clone(),
                        hitten: initial_handle,
                        weapon: String::from("pistol"),
                        point: initial_hit_point,
                    });

                    let fire_message = MessageOut::fire_message(
                        player.id.clone(),
                        event.barrel_origin,
                        event.direction,
                    );
                    server.broadcast_message(DefaultChannel::ReliableOrdered, fire_message.data);
                }
            } else {
                tracing::info!("No hit fire");
                let fire_message = MessageOut::fire_message(
                    player.id.clone(),
                    event.barrel_origin,
                    event.direction,
                );
                server.broadcast_message(DefaultChannel::ReliableOrdered, fire_message.data);
            }
            tracing::info!("Always come here");
        }
    }
}

pub fn handle_hit_events(
    mut hit_events: EventReader<HitEvent>,
    mut query: Query<(&Player, &mut Health)>,
    mut server: ResMut<DenariaServer>,
) {
    for event in hit_events.read() {
        tracing::info!("Hit event {:?}", event);
        if let Ok((player, mut health)) = query.get_mut(event.hitten) {
            tracing::info!("Hit Happened!!");
            health.0 = (health.0 - 20.0).max(0.0);
            let hit_message =
                MessageOut::hit_message(event.hitter_id.clone(), player.id.clone(), event.point);
            server.broadcast_message(DefaultChannel::ReliableOrdered, hit_message.data);
        }
    }
}

pub fn handle_connect_events(
    mut commands: Commands,
    mut connect_events: EventReader<ConnectEvent>,
    mut player_lookup: ResMut<PlayerLookup>,
) {
    for event in connect_events.read() {
        if !player_lookup.map.contains_key(&event.player_id) {
            let initial_translation = Vec3::new(25.0, 20.0, -10.0);
            let entity = commands
                .spawn(PlayerBundle {
                    player: Player {
                        id: event.player_id.clone(),
                    },
                    ..Default::default()
                })
                .insert(RigidBody::KinematicPositionBased)
                .insert(LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z)
                .insert(Collider::capsule_y(0.5, 0.5))
                .insert(ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_STATIC)
                .insert(TransformBundle::from(Transform::from_translation(
                    initial_translation,
                )))
                .insert(KinematicCharacterController {
                    offset: CharacterLength::Absolute(0.01),
                    ..KinematicCharacterController::default()
                })
                .id();

            player_lookup.map.insert(event.player_id.clone(), entity);
        }
    }
}

pub fn handle_disconnect_events(
    mut commands: Commands,
    mut disconnect_events: EventReader<DisconnectEvent>,
    mut player_lookup: ResMut<PlayerLookup>,
    mut server: ResMut<DenariaServer>,
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
