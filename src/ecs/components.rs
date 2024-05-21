use std::collections::HashMap;

use bevy_ecs::{bundle::Bundle, component::Component, entity::Entity, system::Resource};
use rapier3d::{
    dynamics::{RigidBody, RigidBodyHandle},
    geometry::{Collider, ColliderBuilder, ColliderHandle},
};
use serde::{Deserialize, Serialize};

#[derive(Default, Component)]
pub struct Player {
    pub id: String,
}
#[derive(Default, Component)]
pub struct Health(f32);

#[derive(Component)]
pub struct PlayerPhysics {
    pub rigid_body_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub player: Player,
    pub physics: PlayerPhysics,
    pub health: Health,
}

impl Default for PlayerBundle {
    fn default() -> Self {
        PlayerBundle {
            player: Player { id: String::new() },
            physics: PlayerPhysics {
                rigid_body_handle: RigidBodyHandle::default(),
                collider_handle: ColliderHandle::default(),
            },
            health: Health::default(),
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
