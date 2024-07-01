use bevy_ecs::{
    schedule::SystemSet,
    system::{Query, ResMut, Resource},
};
use nalgebra::Vector3;
use rapier3d::{
    control::KinematicCharacterController,
    dynamics::{
        CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet,
        RigidBodySet,
    },
    geometry::{BroadPhaseMultiSap, ColliderSet, NarrowPhase},
    math::Vector,
    pipeline::{PhysicsPipeline, QueryPipeline},
    prelude::*,
};

use crate::ecs::components::{IsGrounded, PlayerPhysics, Position, Rotation, VerticalVelocity};

#[derive(Resource)]
pub struct PhysicsResources {
    pub gravity: Vector<f32>,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhaseMultiSap,
    pub narrow_phase: NarrowPhase,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
    pub character_controller: KinematicCharacterController,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Physics;

pub fn physics_step(mut resources: ResMut<PhysicsResources>) {
    let PhysicsResources {
        gravity,
        integration_parameters,
        physics_pipeline,
        island_manager,
        broad_phase,
        narrow_phase,
        rigid_body_set,
        collider_set,
        impulse_joint_set,
        multibody_joint_set,
        ccd_solver,
        query_pipeline,
        ..
    } = &mut *resources;

    physics_pipeline.step(
        gravity,
        integration_parameters,
        island_manager,
        broad_phase,
        narrow_phase,
        rigid_body_set,
        collider_set,
        impulse_joint_set,
        multibody_joint_set,
        ccd_solver,
        Some(query_pipeline),
        &(),
        &(),
    );
    // query_pipeline.update(rigid_body_set, collider_set);
}

pub fn handle_air_movement(
    mut physics_res: ResMut<PhysicsResources>,
    mut query: Query<(&PlayerPhysics, &mut IsGrounded, &mut VerticalVelocity)>,
) {
    let PhysicsResources {
        rigid_body_set,
        collider_set,
        query_pipeline,
        character_controller,
        ..
    } = &mut *physics_res;

    for (player_physics, mut is_grounded, mut v_velocity) in &mut query {
        if is_grounded.0 {
            v_velocity.0 = 0.0;
        } else {
            v_velocity.0 -= 0.05;
        }
        if v_velocity.0 != 0.0 {
            let dt = 1.0 / 60.0; // Example timestep (assuming 60 FPS)

            let rigid_body_handle = player_physics.rigid_body_handle;
            let collider_handle = player_physics.collider_handle;

            // let current_translation = rigid_body_set[rigid_body_handle].translation();
            let current_position = rigid_body_set[rigid_body_handle].position();
            let collider_shape = collider_set[collider_handle].shape();

            let move_translation = Vector3::new(0.0, v_velocity.0, 0.0);

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
                    .exclude_rigid_body(player_physics.rigid_body_handle), // .exclude_collider(physics.collider_handle),
                |_| {}, // We don’t care about events in this example.
            );

            let rigid_body = rigid_body_set.get_mut(rigid_body_handle).unwrap();

            is_grounded.0 = corrected_movement.grounded;

            rigid_body.set_next_kinematic_translation(
                rigid_body.translation() + corrected_movement.translation,
            );
        }
    }
}

pub fn update_physic_components(
    mut resources: ResMut<PhysicsResources>,
    mut query: Query<(&PlayerPhysics, &mut Position, &mut Rotation)>,
) {
    let PhysicsResources { rigid_body_set, .. } = &mut *resources;

    for (player_physics, mut position, mut rotation) in &mut query {
        let player_rigid_body = rigid_body_set
            .get(player_physics.rigid_body_handle)
            .unwrap();

        let rigid_body_translation = player_rigid_body.translation();
        let unit_quaternion = player_rigid_body.rotation();

        if &position.0 != rigid_body_translation {
            position.0 = *rigid_body_translation;
        }

        if rotation.0 != *unit_quaternion {
            rotation.0 = *unit_quaternion;
        }
    }
}
