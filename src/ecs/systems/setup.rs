use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Instant, SystemTime},
    vec,
};

use bevy_ecs::{
    event::Events,
    system::{Commands, ResMut, Resource},
};
use rapier3d::{
    control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
    na::Vector3,
    prelude::*,
};
use serde::Serialize;

use crate::{
    ecs::{
        components::{ColliderHandleLookup, PlayerLookup},
        events::{
            ConnectEvent, DisconnectEvent, FireEvent, HitEvent, JumpEvent, LookEvent, MoveEvent,
        },
        systems::physics::PhysicsResources,
    },
    server::{
        channel::DefaultChannel,
        connection::ConnectionConfig,
        message_out::MessageOut,
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
    let instant = InstantResource::default();

    let mut character_controller = KinematicCharacterController::default();
    character_controller.offset = CharacterLength::Absolute(0.01);
    character_controller.snap_to_ground = Some(CharacterLength::Absolute(0.05));
    character_controller.autostep = Some(CharacterAutostep {
        max_height: CharacterLength::Absolute(0.05),
        min_width: CharacterLength::Absolute(0.2),
        include_dynamic_bodies: false,
    });

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

    let objects: Vec<LevelObject> = vec![];

    let level_objects = LevelObjects { objects };

    commands.insert_resource(server);
    commands.insert_resource(transport);
    commands.insert_resource(PlayerLookup::new());
    commands.insert_resource(ColliderHandleLookup::new());
    commands.insert_resource(instant);
    commands.insert_resource(physics_res);
    commands.insert_resource(level_objects);

    commands.insert_resource(Events::<ConnectEvent>::default());
    commands.insert_resource(Events::<DisconnectEvent>::default());
    commands.insert_resource(Events::<MoveEvent>::default());
    commands.insert_resource(Events::<LookEvent>::default());
    commands.insert_resource(Events::<JumpEvent>::default());
    commands.insert_resource(Events::<FireEvent>::default());
    commands.insert_resource(Events::<HitEvent>::default());
}

pub fn setup_level(
    mut physics_res: ResMut<PhysicsResources>,
    mut level_objects: ResMut<LevelObjects>,
) {
    let PhysicsResources {
        rigid_body_set,
        collider_set,
        ..
    } = &mut *physics_res;

    let terrain_object = LevelObject::new_cuboid(
        rigid_body_set,
        collider_set,
        vector![500.0, 0.1, 500.0],
        vector![500.0, 0.0, 500.0],
        LevelObjectColor::Gray,
    );

    let cube_object = LevelObject::new_cuboid(
        rigid_body_set,
        collider_set,
        vector![100.0, 0.5, 100.0],
        vector![100.0, 0.5, 100.0],
        LevelObjectColor::Green,
    );

    let cube2_object = LevelObject::new_cuboid(
        rigid_body_set,
        collider_set,
        vector![10.0, 5.0, 3.0],
        vector![10.0, 5.0, 30.0],
        LevelObjectColor::Red,
    );

    let player_object = LevelObject::new_capsule(
        rigid_body_set,
        collider_set,
        vector![1.0, 1.0, 1.0],
        vector![5.0, 3.0, 25.0],
        LevelObjectColor::Blue,
    );

    level_objects.objects.push(terrain_object);
    level_objects.objects.push(cube_object);
    level_objects.objects.push(cube2_object);
    level_objects.objects.push(player_object);

    let map_width = 200.0;
    let map_height = 200.0;
    let wall_height = 50.0;
    let wall_thickness = 1.0;

    let edge_objects = LevelObject::new_edges(
        rigid_body_set,
        collider_set,
        map_width,
        map_height,
        wall_height,
        wall_thickness,
    );

    level_objects.objects.extend(edge_objects);
}

pub fn send_level_objects(
    server: &mut MattaServer,
    level_objects: &LevelObjects,
    player_id: String,
) {
    let level_objects_message =
        MessageOut::level_objects_message(level_objects.objects.clone()).unwrap();

    let client_id = server.client_id_by_player_id(player_id).unwrap();

    server.send_message(
        client_id,
        DefaultChannel::ReliableOrdered,
        level_objects_message.data,
    )
}

#[derive(Debug, Resource, Serialize)]
pub struct LevelObjects {
    objects: Vec<LevelObject>,
}

// Level Object size format uses the convention of Unity3D Game Engine's scale
#[derive(Debug, Serialize, Clone)]
pub struct LevelObject {
    // Ball: 0, Cube: 1, Capsule: 2
    object_type: u8,
    color: u8,
    translation: Vector3<f32>,
    size: Vector3<f32>,
}

impl LevelObject {
    fn new_cuboid(
        rigid_body_set: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        size: Vector3<f32>,
        translation: Vector3<f32>,
        color: LevelObjectColor,
    ) -> LevelObject {
        let rigid_body = RigidBodyBuilder::new(RigidBodyType::Fixed)
            .translation(translation)
            .build();
        let rigid_body_handle = rigid_body_set.insert(rigid_body);

        let collider = ColliderBuilder::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0).build();

        let _collider_handle =
            collider_set.insert_with_parent(collider, rigid_body_handle, rigid_body_set);

        LevelObject {
            object_type: 1,
            color: color as u8,
            translation,
            size,
        }
    }

    fn new_capsule(
        rigid_body_set: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        size: Vector3<f32>,
        translation: Vector3<f32>,
        color: LevelObjectColor,
    ) -> LevelObject {
        let rigid_body = RigidBodyBuilder::new(RigidBodyType::Fixed)
            .translation(translation)
            .build();
        let rigid_body_handle = rigid_body_set.insert(rigid_body);

        let collider = ColliderBuilder::capsule_y(size.y / 2.0, size.x / 2.0).build();

        let _collider_handle =
            collider_set.insert_with_parent(collider, rigid_body_handle, rigid_body_set);

        LevelObject {
            object_type: 2,
            color: color as u8,
            translation,
            size,
        }
    }

    fn new_edges(
        rigid_body_set: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        map_width: f32,
        map_height: f32,
        wall_height: f32,
        wall_thickness: f32,
    ) -> Vec<LevelObject> {
        let front_object = LevelObject::new_cuboid(
            rigid_body_set,
            collider_set,
            vector![map_width, wall_height, wall_thickness],
            vector![map_width / 2.0, wall_height / 2.0, 0.0],
            LevelObjectColor::None,
        );

        let left_object = LevelObject::new_cuboid(
            rigid_body_set,
            collider_set,
            vector![wall_thickness, wall_height, map_height],
            vector![0.0, wall_height / 2.0, map_height / 2.0],
            LevelObjectColor::None,
        );

        let back_object = LevelObject::new_cuboid(
            rigid_body_set,
            collider_set,
            vector![map_width, wall_height, wall_thickness],
            vector![map_width / 2.0, wall_height / 2.0, map_height],
            LevelObjectColor::White,
        );

        let rigth_object = LevelObject::new_cuboid(
            rigid_body_set,
            collider_set,
            vector![wall_thickness, wall_height, map_height],
            vector![map_width, wall_height / 2.0, map_height / 2.0],
            LevelObjectColor::White,
        );

        let ceiling_object = LevelObject::new_cuboid(
            rigid_body_set,
            collider_set,
            vector![map_width, wall_thickness, map_height],
            vector![map_width / 2.0, wall_height, map_height / 2.0],
            LevelObjectColor::None,
        );

        let ground_object = LevelObject::new_cuboid(
            rigid_body_set,
            collider_set,
            vector![map_width, wall_thickness, map_height],
            vector![map_width / 2.0, 0.0, map_height / 2.0],
            LevelObjectColor::White,
        );

        let edges = vec![
            front_object,
            left_object,
            back_object,
            rigth_object,
            ceiling_object,
            ground_object,
        ];
        edges
    }
}

#[repr(u8)]
enum LevelObjectColor {
    None = 0,  // 0
    Red = 1,   // 1
    Green = 2, // 2
    Blue = 3,  // 3
    White = 4, // 4
    Black = 5, // 5
    Gray = 6,  // 6
}

#[derive(Resource)]
pub struct InstantResource(pub Instant);

impl Default for InstantResource {
    fn default() -> Self {
        InstantResource(Instant::now())
    }
}

struct MapEdgesPositions {
    front: Vector3<f32>,
    left: Vector3<f32>,
    back: Vector3<f32>,
    right: Vector3<f32>,
    ceiling: Vector3<f32>,
    ground: Vector3<f32>,
}

struct MapEdge {
    position: Vector3<f32>,
    scale: Vector3<f32>,
}
