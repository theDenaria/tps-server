use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use bevy_ecs::{
    event::Events,
    system::{Commands, Resource},
};
use rapier3d::{
    control::{CharacterLength, KinematicCharacterController},
    prelude::*,
};

use crate::{
    ecs::{
        components::PlayerLookup,
        events::{ConnectEvent, DisconnectEvent, FireEvent, JumpEvent, LookEvent, MoveEvent},
        systems::physics::PhysicsResources,
    },
    server::{
        connection::ConnectionConfig,
        server::MattaServer,
        transport::{server::server::ServerConfig, transport::ServerTransport},
    },
};

pub fn setup(mut commands: Commands) {
    let server = MattaServer::new(ConnectionConfig::default());
    // Setup transport layer
    const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
    let socket: UdpSocket = UdpSocket::bind(SERVER_ADDR).unwrap();
    let server_config = ServerConfig {
        current_time: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap(),
        max_clients: 64,
        public_addresses: vec![SERVER_ADDR],
    };
    let transport = ServerTransport::new(server_config, socket).unwrap();
    let duration = DurationResource::default();

    let mut character_controller = KinematicCharacterController::default();
    character_controller.offset = CharacterLength::Absolute(0.01);
    character_controller.snap_to_ground = Some(CharacterLength::Absolute(0.5));

    let physics_res = PhysicsResources {
        gravity: vector![0.0, -9.81, 0.0],
        integration_parameters: IntegrationParameters::default(),
        physics_pipeline: PhysicsPipeline::new(),
        island_manager: IslandManager::new(),
        broad_phase: BroadPhaseMultiSap::new(),
        narrow_phase: NarrowPhase::new(),
        rigid_body_set: RigidBodySet::new(),
        collider_set: ColliderSet::new(),
        impulse_joint_set: ImpulseJointSet::new(),
        multibody_joint_set: MultibodyJointSet::new(),
        ccd_solver: CCDSolver::new(),
        query_pipeline: QueryPipeline::new(),
        character_controller,
    };

    commands.insert_resource(character_controller);

    commands.insert_resource(server);
    commands.insert_resource(transport);
    commands.insert_resource(PlayerLookup::new());
    commands.insert_resource(duration);

    commands.insert_resource(Events::<ConnectEvent>::default());
    commands.insert_resource(Events::<DisconnectEvent>::default());
    commands.insert_resource(Events::<MoveEvent>::default());
    commands.insert_resource(Events::<LookEvent>::default());
    commands.insert_resource(Events::<JumpEvent>::default());
    commands.insert_resource(Events::<FireEvent>::default());
}

#[derive(Resource)]
pub struct DurationResource(pub Duration);

impl Default for DurationResource {
    fn default() -> Self {
        DurationResource(Duration::default())
    }
}
