use bevy_ecs::{entity::Entity, event::Event};
use rapier3d::na::{Point3, Vector3};

#[derive(Debug, Event)]
pub struct MoveEvent {
    pub entity: Entity,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Event)]
pub struct LookEvent {
    pub entity: Entity,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
#[derive(Event)]
pub struct JumpEvent {
    pub entity: Entity,
}

#[derive(Event)]
pub struct FireEvent {
    pub entity: Entity,
    pub cam_origin: Point3<f32>,
    pub direction: Vector3<f32>,
    pub barrel_origin: Point3<f32>,
}

#[derive(Event)]
pub struct HitEvent {
    pub hitter_id: String,
    pub hitten: Entity,
    pub weapon: String,
    pub point: Vector3<f32>,
}

#[derive(Event)]
pub struct ConnectEvent {
    pub player_id: String,
}

#[derive(Event)]
pub struct DisconnectEvent {
    pub player_id: String,
}
