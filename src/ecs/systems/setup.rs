use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, Instant, SystemTime},
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

    let terrain_translation = vector![500.0, 0.0, 500.0];
    let terrain_rigid_body = RigidBodyBuilder::new(RigidBodyType::Fixed)
        // The rigid body translation.
        // Default: zero vector.
        .translation(terrain_translation)
        // All done, actually build the rigid-body.
        .build();
    let terrain_size = vector![500.0, 0.1, 500.0];
    let terrain_collider = ColliderBuilder::cuboid(terrain_size.x, terrain_size.y, terrain_size.z)
        // .active_collision_types(
        //     ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_FIXED,
        // )
        .build();

    let terrain_rigid_body_handle = rigid_body_set.insert(terrain_rigid_body);

    let _terrain_collider_handle = collider_set.insert_with_parent(
        terrain_collider,
        terrain_rigid_body_handle,
        rigid_body_set,
    );

    let cube_translation = vector![100.0, 0.5, 100.0];
    let cube_rigid_body = RigidBodyBuilder::new(RigidBodyType::Fixed)
        // The rigid body translation.
        // Default: zero vector.
        .translation(cube_translation)
        // All done, actually build the rigid-body.
        .build();
    let cube_size = vector![100.0, 0.5, 100.0];
    let cube_collider = ColliderBuilder::cuboid(cube_size.x, cube_size.y, cube_size.z)
        // .active_collision_types(
        //     ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_FIXED,
        // )
        .build();

    let cube_rigid_body_handle = rigid_body_set.insert(cube_rigid_body);

    let _cube_collider_handle =
        collider_set.insert_with_parent(cube_collider, cube_rigid_body_handle, rigid_body_set);

    let cube2_translation = vector![10.0, 5.0, 30.0];
    let cube2_rigid_body = RigidBodyBuilder::new(RigidBodyType::Fixed)
        // The rigid body translation.
        // Default: zero vector.
        .translation(cube2_translation)
        // All done, actually build the rigid-body.
        .build();
    let cube2_size = vector![10.0, 5.0, 3.0];
    let cube2_collider =
        ColliderBuilder::cuboid(cube2_size.x / 2.0, cube2_size.y / 2.0, cube2_size.z / 2.0)
            // .active_collision_types(
            //     ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_FIXED,
            // )
            .build();

    let cube2_rigid_body_handle = rigid_body_set.insert(cube2_rigid_body);

    let _cube2_collider_handle =
        collider_set.insert_with_parent(cube2_collider, cube2_rigid_body_handle, rigid_body_set);

    let player_translation = vector![5.0, 3.0, 25.0];
    let player_rigid_body = RigidBodyBuilder::new(RigidBodyType::Fixed)
        // The rigid body translation.
        // Default: zero vector.
        .translation(player_translation)
        // All done, actually build the rigid-body.
        .build();
    let player_size = vector![1.0, 1.0, 1.0];
    let player_collider = ColliderBuilder::capsule_y(player_size.x / 2.0, player_size.y / 2.0)
        // .active_collision_types(
        //     ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_FIXED,
        // )
        .build();

    let player_rigid_body_handle = rigid_body_set.insert(player_rigid_body);

    let _player_collider_handle =
        collider_set.insert_with_parent(player_collider, player_rigid_body_handle, rigid_body_set);

    let terrain_level_object = LevelObject {
        object_type: 1,
        translation: terrain_translation,
        size: terrain_size,
    };

    let cube_level_object = LevelObject {
        object_type: 1,
        translation: cube_translation,
        size: cube_size,
    };

    let cube2_level_object = LevelObject {
        object_type: 1,
        translation: cube2_translation,
        size: cube2_size,
    };

    let player_level_object = LevelObject {
        object_type: 2,
        translation: player_translation,
        size: player_size,
    };

    level_objects.objects.push(terrain_level_object);
    level_objects.objects.push(cube_level_object);
    level_objects.objects.push(cube2_level_object);
    level_objects.objects.push(player_level_object);

    let map_width = 200.0;
    let map_height = 200.0;
    let wall_height = 50.0;
    let wall_thickness = 1.0;
    let rotation_degrees: f32 = 45.0; // Example rotation

    // Convert degrees to radians
    let rotation_radians = rotation_degrees.to_radians();

    let map_edges = vec![
        MapEdge {
            // front
            position: Vector3::new(map_width / 2.0, wall_height / 2.0, 0.0),
            scale: Vector3::new(map_width, wall_height, wall_thickness),
        },
        MapEdge {
            // left
            position: Vector3::new(0.0, wall_height / 2.0, map_height / 2.0),
            scale: Vector3::new(wall_thickness, wall_height, map_height),
        },
        MapEdge {
            // back
            position: Vector3::new(map_width / 2.0, wall_height / 2.0, map_height),
            scale: Vector3::new(map_width, wall_height, wall_thickness),
        },
        MapEdge {
            // right
            position: Vector3::new(map_width, wall_height / 2.0, map_height / 2.0),
            scale: Vector3::new(wall_thickness, wall_height, map_height),
        },
        MapEdge {
            // ceiling
            position: Vector3::new(map_width / 2.0, wall_height, map_height / 2.0),
            scale: Vector3::new(map_width, wall_thickness, map_height),
        },
        MapEdge {
            // ground
            position: Vector3::new(map_width / 2.0, 0.0, map_height / 2.0),
            scale: Vector3::new(map_width, wall_thickness, map_height),
        },
    ];

    for edge in &map_edges {
        let wall_shape =
            ColliderBuilder::cuboid(edge.scale.x / 2.0, edge.scale.y / 2.0, edge.scale.z / 2.0)
                .translation(edge.position)
                .build();
        collider_set.insert(wall_shape);

        let edge_level_object = LevelObject {
            object_type: 9,
            translation: edge.position,
            size: edge.scale,
        };

        level_objects.objects.push(edge_level_object);
    }
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

#[derive(Debug, Serialize, Clone)]
pub struct LevelObject {
    // Ball: 0, Cube: 1, Capsule: 2
    object_type: u8,
    translation: Vector3<f32>,
    size: Vector3<f32>,
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
