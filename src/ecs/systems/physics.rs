use bevy_ecs::{
    schedule::SystemSet,
    system::{ResMut, Resource},
};
use rapier3d::{
    control::KinematicCharacterController,
    dynamics::{
        CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet,
        RigidBodySet,
    },
    geometry::{BroadPhaseMultiSap, ColliderSet, NarrowPhase},
    math::Vector,
    pipeline::{PhysicsPipeline, QueryPipeline},
};

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

pub fn physics_step<'a>(mut resources: ResMut<PhysicsResources>) {
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
}
