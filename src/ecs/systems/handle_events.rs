use bevy_ecs::{
    event::{EventReader, EventWriter},
    schedule::SystemSet,
    system::{Commands, Query, Res, ResMut},
};

use nalgebra::{Quaternion, UnitQuaternion};
use rapier3d::prelude::*;

use crate::{
    constants::VELOCITY_MUL,
    ecs::{
        components::{
            ColliderHandleLookup, Health, IsGrounded, Player, PlayerBundle, PlayerLookup,
            PlayerPhysics, VerticalVelocity,
        },
        events::{
            ConnectEvent, DisconnectEvent, FireEvent, HitEvent, JumpEvent, LookEvent, MoveEvent,
        },
    },
    server::{channel::DefaultChannel, message_out::MessageOut, server::MattaServer},
};

use super::{
    physics::PhysicsResources,
    setup::{send_level_objects, LevelObjects},
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HandleGameEvents;

pub fn handle_move_events(
    mut move_events: EventReader<MoveEvent>,
    mut query: Query<(&Player, &PlayerPhysics, &mut IsGrounded)>,
    mut physics_res: ResMut<PhysicsResources>,
) {
    for event in move_events.read() {
        let PhysicsResources {
            rigid_body_set,
            collider_set,
            query_pipeline,
            character_controller,
            ..
        } = &mut *physics_res;
        if let Ok((_player, physics, mut grounded)) = query.get_mut(event.entity) {
            let rigid_body_handle = physics.rigid_body_handle;
            let collider_handle = physics.collider_handle;

            let _current_translation = rigid_body_set[rigid_body_handle].translation();
            let current_position = rigid_body_set[rigid_body_handle].position();
            let collider_shape = collider_set[collider_handle].shape();

            let direction = vector![event.x, 0.0, event.y];
            let normalized_direction = if direction.magnitude() > 0.0 {
                direction.normalize()
            } else {
                direction
            };

            let dt = 1.0 / 60.0; // Example timestep (assuming 60 FPS)

            let move_translation = normalized_direction * VELOCITY_MUL * dt * 2.0;

            let corrected_movement = character_controller.move_shape(
                dt,               // The timestep length (can be set to SimulationSettings::dt).
                rigid_body_set,   // The RigidBodySet.
                collider_set,     // The ColliderSet.
                query_pipeline,   // The QueryPipeline.
                collider_shape,   // The character’s shape.
                current_position, // The character’s initial position.
                move_translation,
                QueryFilter::default()
                    // Make sure the character we are trying to move isn’t considered an obstacle.
                    .exclude_rigid_body(physics.rigid_body_handle), // .exclude_collider(physics.collider_handle),
                |_| {}, // We don’t care about events in this example.
            );

            let rigid_body = rigid_body_set.get_mut(physics.rigid_body_handle).unwrap();

            grounded.0 = corrected_movement.grounded;

            rigid_body.set_next_kinematic_translation(
                rigid_body.translation() + corrected_movement.translation,
            );
        }
    }
}

pub fn handle_look_events(
    mut look_events: EventReader<LookEvent>,
    query: Query<&PlayerPhysics>,
    mut physics_res: ResMut<PhysicsResources>,
) {
    for event in look_events.read() {
        if let Ok(physics) = query.get(event.entity) {
            let PhysicsResources { rigid_body_set, .. } = &mut *physics_res;
            let rigid_body = rigid_body_set.get_mut(physics.rigid_body_handle).unwrap();

            let quaternion_data = Quaternion::new(event.w, event.x, event.y, event.z);

            let rot_quaternion = UnitQuaternion::from_quaternion(quaternion_data);

            rigid_body.set_next_kinematic_rotation(rot_quaternion);
        }
    }
}

pub fn handle_jump_events(
    mut jump_events: EventReader<JumpEvent>,
    mut query: Query<(&mut IsGrounded, &mut VerticalVelocity)>,
) {
    for event in jump_events.read() {
        if let Ok((mut is_grounded, mut v_velocity)) = query.get_mut(event.entity) {
            if is_grounded.0 {
                v_velocity.0 += 0.5;
                is_grounded.0 = false;
            }
        }
    }
}

pub fn handle_fire_events(
    mut fire_events: EventReader<FireEvent>,
    mut physics_res: ResMut<PhysicsResources>,
    collider_lookup: ResMut<ColliderHandleLookup>,
    query: Query<(&Player, &PlayerPhysics)>,
    mut hit_event: EventWriter<HitEvent>,
    mut server: ResMut<MattaServer>,
) {
    let PhysicsResources {
        rigid_body_set,
        collider_set,
        query_pipeline,
        ..
    } = &mut *physics_res;

    query_pipeline.update(&rigid_body_set, &collider_set);
    let max_toi = Real::MAX;
    let solid = true;

    for event in fire_events.read() {
        if let Ok((player, player_physics)) = query.get(event.entity) {
            let camera_ray = Ray::new(event.cam_origin, event.direction);
            if let Some((initial_handle, initial_toi)) = query_pipeline.cast_ray(
                &rigid_body_set,
                &collider_set,
                &camera_ray,
                max_toi,
                solid,
                QueryFilter::default().exclude_collider(player_physics.collider_handle),
            ) {
                let initial_hit_point = camera_ray.point_at(initial_toi);
                tracing::info!("Initial hit point: {:?}", initial_hit_point);

                // Second raycast from the barrel position to the initial hit point
                let barrel_target_dir = (initial_hit_point - event.barrel_origin).normalize();

                let dot_product = camera_ray.dir.dot(&barrel_target_dir);
                let magnitude_product = camera_ray.dir.magnitude() * barrel_target_dir.magnitude();
                let cosine_angle = dot_product / magnitude_product;

                // Clamp the cosine value to avoid potential NaN due to floating-point precision
                let clamped_cosine = cosine_angle.clamp(-1.0, 1.0);

                // Calculate the angle in radians and then convert to degrees
                let angle = clamped_cosine.acos().to_degrees();

                // Define the angle threshold
                let angle_threshold = 70.0;

                tracing::info!("Angle: {:?}, Threshold: {:?}", angle, angle_threshold);

                if angle <= angle_threshold {
                    let barrel_ray = Ray::new(event.barrel_origin, barrel_target_dir.clone());
                    if let Some((handle, toi)) = query_pipeline.cast_ray(
                        &rigid_body_set,
                        &collider_set,
                        &barrel_ray,
                        max_toi,
                        solid,
                        QueryFilter::default().exclude_collider(player_physics.collider_handle),
                    ) {
                        let hit_point = barrel_ray.point_at(toi);
                        tracing::info!("Main target or an obstacle hit");
                        if let Some(hit_entity) = collider_lookup.map.get(&handle) {
                            hit_event.send(HitEvent {
                                hitter_id: player.id.clone(),
                                hitten: *hit_entity,
                                weapon: String::from("pistol"),
                                point: hit_point.coords,
                            });
                        }

                        let fire_message = MessageOut::fire_message(
                            player.id.clone(),
                            event.barrel_origin.coords,
                            barrel_target_dir,
                        );
                        server
                            .broadcast_message(DefaultChannel::ReliableOrdered, fire_message.data);
                    }
                } else {
                    // No obstacle between the barrel and the target, so use the initial hit point
                    if let Some(hit_entity) = collider_lookup.map.get(&initial_handle) {
                        tracing::info!("Main target threshold misses");
                        hit_event.send(HitEvent {
                            hitter_id: player.id.clone(),
                            hitten: *hit_entity,
                            weapon: String::from("pistol"),
                            point: initial_hit_point.coords,
                        });
                    }

                    let fire_message = MessageOut::fire_message(
                        player.id.clone(),
                        event.barrel_origin.coords,
                        event.direction,
                    );
                    server.broadcast_message(DefaultChannel::ReliableOrdered, fire_message.data);
                }
            } else {
                tracing::info!("No hit fire");
                let fire_message = MessageOut::fire_message(
                    player.id.clone(),
                    event.barrel_origin.coords,
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
    mut server: ResMut<MattaServer>,
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
    mut collider_lookup: ResMut<ColliderHandleLookup>,
    mut physics_res: ResMut<PhysicsResources>,
    mut server: ResMut<MattaServer>,
    level_objects: Res<LevelObjects>,
) {
    let PhysicsResources {
        rigid_body_set,
        collider_set,
        ..
    } = &mut *physics_res;

    for event in connect_events.read() {
        if !player_lookup.map.contains_key(&event.player_id) {
            let initial_translation = vector![5.0, 5.0, 5.0];
            let rigid_body = RigidBodyBuilder::new(RigidBodyType::KinematicPositionBased)
                // The rigid body translation.
                // Default: zero vector.
                .translation(initial_translation.clone())
                .enabled_rotations(false, true, false)
                // All done, actually build the rigid-body.
                .build();

            let collider = ColliderBuilder::capsule_y(0.5, 0.5)
                .active_collision_types(
                    ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_FIXED,
                )
                .build();

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
            collider_lookup.map.insert(collider_handle, entity);

            send_level_objects(&mut server, &level_objects, event.player_id.clone());
        }
    }
}

pub fn handle_disconnect_events(
    mut commands: Commands,
    mut disconnect_events: EventReader<DisconnectEvent>,
    mut player_lookup: ResMut<PlayerLookup>,
    mut collider_lookup: ResMut<ColliderHandleLookup>,
    query: Query<&PlayerPhysics>,
    mut server: ResMut<MattaServer>,
) {
    if disconnect_events.len() > 0 {
        let mut disconnect_player_ids: Vec<&String> = vec![];
        for event in disconnect_events.read() {
            if let Some(entity) = player_lookup.map.get(&event.player_id) {
                if let Ok(player_physics) = query.get(*entity) {
                    collider_lookup.map.remove(&player_physics.collider_handle);
                }
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
