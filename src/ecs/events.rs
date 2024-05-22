use bevy_ecs::{entity::Entity, event::Event};
use rapier3d::{
    math::{Real, Vector},
    na::Vector3,
};

#[derive(Event)]
pub struct MoveEvent {
    pub entity: Entity,
    pub x: f32,
    pub y: f32,
}

#[derive(Event)]
pub struct LookEvent {
    pub entity: Entity,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Event)]
pub struct PositionChangeEvent {
    pub player_id: String,
    pub translation: Vector<Real>,
}

#[derive(Event)]
pub struct RotationChangeEvent {
    pub player_id: String,
    pub rotation: Vector<Real>,
}

#[derive(Event)]
pub struct JumpEvent {
    pub entity: Entity,
}

#[derive(Event)]
pub struct FireEvent {
    pub entity: Entity,
    pub origin: Vector3<f32>,
    pub direction: Vector3<f32>,
}

#[derive(Event)]
pub struct ConnectEvent {
    pub player_id: String,
}

#[derive(Event)]
pub struct DisconnectEvent {
    pub player_id: String,
}
