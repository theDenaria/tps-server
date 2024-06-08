use bevy_ecs::event::EventWriter;
use rapier3d::na::{Point3, Vector3};

use crate::ecs::{
    components::PlayerLookup,
    events::{ConnectEvent, DisconnectEvent, FireEvent, JumpEvent, LookEvent, MoveEvent},
};

pub fn send_move_event(
    player_id: &String,
    move_x: f32,
    move_y: f32,
    player_lookup: &PlayerLookup,
    move_event: &mut EventWriter<MoveEvent>,
) {
    if let Some(player_entity) = player_lookup.map.get(player_id) {
        move_event.send(MoveEvent {
            entity: *player_entity,
            x: move_x,
            y: move_y,
        });
    } else {
        tracing::warn!("Player ID not found: {}", player_id);
    }
}

pub fn send_look_event(
    player_id: &String,
    look_x: f32,
    look_y: f32,
    look_z: f32,
    look_w: f32,
    player_lookup: &PlayerLookup,
    look_event: &mut EventWriter<LookEvent>,
) {
    if let Some(player_entity) = player_lookup.map.get(player_id) {
        look_event.send(LookEvent {
            entity: *player_entity,
            x: look_x,
            y: look_y,
            z: look_z,
            w: look_w,
        });
    } else {
        tracing::warn!("Player ID not found: {}", player_id);
    }
}

pub fn send_jump_event(
    player_id: &String,
    player_lookup: &PlayerLookup,
    jump_event: &mut EventWriter<JumpEvent>,
) {
    if let Some(player_entity) = player_lookup.map.get(player_id) {
        jump_event.send(JumpEvent {
            entity: *player_entity,
        });
    } else {
        tracing::warn!("Player ID not found: {}", player_id);
    }
}

pub fn send_fire_event(
    player_id: &String,
    cam_origin: Point3<f32>,
    direction: Vector3<f32>,
    barrel_origin: Point3<f32>,
    player_lookup: &PlayerLookup,
    fire_event: &mut EventWriter<FireEvent>,
) {
    if let Some(player_entity) = player_lookup.map.get(player_id) {
        fire_event.send(FireEvent {
            entity: *player_entity,
            cam_origin,
            direction,
            barrel_origin,
        });
    } else {
        tracing::warn!("Player ID not found: {}", player_id);
    }
}

pub fn send_connect_event(player_id: &String, connect_event: &mut EventWriter<ConnectEvent>) {
    connect_event.send(ConnectEvent {
        player_id: player_id.clone(),
    });
}

pub fn send_disconnect_event(
    player_id: String,
    disconnect_event: &mut EventWriter<DisconnectEvent>,
) {
    disconnect_event.send(DisconnectEvent { player_id });
}