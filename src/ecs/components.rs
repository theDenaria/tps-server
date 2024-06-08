use std::collections::HashMap;

use bevy_ecs::{bundle::Bundle, component::Component, entity::Entity, system::Resource};
use rapier3d::{
    dynamics::RigidBodyHandle,
    geometry::ColliderHandle,
    na::{UnitQuaternion, Vector3},
};

#[derive(Default, Component)]
pub struct Player {
    pub id: String,
}
#[derive(Default, Component)]
pub struct Health(pub f32);

#[derive(Debug, Component)]
pub struct PlayerPhysics {
    pub rigid_body_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
}

#[derive(Debug, Component)]
pub struct Position(pub Vector3<f32>);

#[derive(Debug, Component)]
pub struct Rotation(pub UnitQuaternion<f32>);

#[derive(Debug, Component)]
pub struct IsGrounded(pub bool);

#[derive(Debug, Component)]
pub struct FireOn(pub bool);

#[derive(Debug, Component)]
pub struct VerticalVelocity(pub f32);

#[derive(Bundle)]
pub struct PlayerBundle {
    pub player: Player,
    pub physics: PlayerPhysics,
    pub position: Position,
    pub rotation: Rotation,
    pub health: Health,
    pub grounded: IsGrounded,
    pub fire_on: FireOn,
    pub v_velocity: VerticalVelocity,
}

impl Default for PlayerBundle {
    fn default() -> Self {
        PlayerBundle {
            player: Player { id: String::new() },
            physics: PlayerPhysics {
                rigid_body_handle: RigidBodyHandle::default(),
                collider_handle: ColliderHandle::default(),
            },
            position: Position(Vector3::default()),
            rotation: Rotation(UnitQuaternion::default()),
            health: Health(100.0),
            grounded: IsGrounded(false),
            fire_on: FireOn(false),
            v_velocity: VerticalVelocity(0.0),
        }
    }
}

#[derive(Resource)]
pub struct PlayerLookup {
    pub map: HashMap<String, Entity>,
}

impl PlayerLookup {
    pub fn new() -> PlayerLookup {
        PlayerLookup {
            map: HashMap::new(),
        }
    }
}

#[derive(Resource)]
pub struct ColliderHandleLookup {
    pub map: HashMap<ColliderHandle, Entity>,
}

impl ColliderHandleLookup {
    pub fn new() -> ColliderHandleLookup {
        ColliderHandleLookup {
            map: HashMap::new(),
        }
    }
}
