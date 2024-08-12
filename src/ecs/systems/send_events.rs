use bevy::prelude::*;

use crate::ecs::{
    components::PlayerLookup,
    events::{ConnectEvent, DisconnectEvent, FireEvent, LookEvent},
};

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
            direction: Vec4::new(look_x, look_y, look_z, look_w),
        });
    } else {
        tracing::warn!("Player ID not found: {}", player_id);
    }
}

pub fn send_fire_event(
    player_id: &String,
    cam_origin: Vec3,
    direction: Vec3,
    barrel_origin: Vec3,
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
