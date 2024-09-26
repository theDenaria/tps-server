use bevy::prelude::*;

#[derive(Debug, Event)]
pub struct MoveEvent {
    pub entity: Entity,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Event)]
pub struct LookEvent {
    pub entity: Entity,
    pub direction: Vec4,
}
#[derive(Event)]
pub struct JumpEvent {
    pub entity: Entity,
}

#[derive(Event)]
pub struct FireEvent {
    pub entity: Entity,
    pub cam_origin: Vec3,
    pub direction: Vec3,
    pub barrel_origin: Vec3,
}

#[derive(Event, Debug)]
pub struct HitEvent {
    pub hitter_id: String,
    pub hitten: Entity,
    pub weapon: String,
    pub point: Vec3,
}

#[derive(Event)]
pub struct ConnectEvent {
    pub player_id: String,
}

#[derive(Event)]
pub struct DisconnectEvent {
    pub player_id: String,
}
