use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::{
    app::ScheduleRunnerPlugin,
    diagnostic::{
        EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::*,
};
use bevy_rapier3d::{
    plugin::{NoUserData, RapierPhysicsPlugin},
    render::RapierDebugRenderPlugin,
};
use iyes_perf_ui::PerfUiPlugin;

use crate::{
    ecs::{
        events::ConnectEvent,
        systems::{
            debug::{
                look_debug_camera, move_debug_camera, set_debug_3d_render_camera,
                set_debug_metrics, set_debug_metrics_cam,
            },
            handle_events::{
                handle_character_movement, handle_connect_events, handle_disconnect_events,
                handle_fire_events, handle_hit_events, handle_look_events,
            },
            handle_server::{handle_server_events, transport_send_packets},
            on_change::{on_health_change, on_transform_change},
            setup::{setup, setup_level},
        },
    },
    server::{server::DenariaServer, transport::transport::ServerTransport},
};

pub struct Session {
    pub id: u64,
    pub player_map: HashMap<String, u8>,
    pub app: App,
}

impl Session {
    pub fn new(id: u64, player_map: HashMap<String, u8>) -> Self {
        Session {
            id,
            player_map,
            app: App::new(),
        }
    }
}

pub struct SessionHandler {
    sessions: Arc<Mutex<HashMap<u64, Session>>>,
    player_session_map: Arc<Mutex<HashMap<String, u64>>>,
}

#[derive(Resource, Clone)]
pub struct NetworkResource {
    pub server: Arc<Mutex<DenariaServer>>,
    pub transport: Arc<Mutex<ServerTransport>>,
}

impl SessionHandler {
    pub fn new() -> Self {
        SessionHandler {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            player_session_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn new_session(&mut self, input: SessionCreateInput, network_resource: NetworkResource) {
        let mut sess = self.sessions.lock().unwrap();
        let player_map: HashMap<String, u8> = input.players.into_iter().collect();

        sess.insert(input.id, Session::new(input.id, player_map));

        if let Some(session) = sess.get_mut(&input.id) {
            let enable_debug_metrics =
                std::env::var("ENABLE_DEBUG_METRICS").is_ok_and(|v| v.to_lowercase() == "true");
            let enable_debug_cam =
                std::env::var("ENABLE_DEBUG_CAM").is_ok_and(|v| v.to_lowercase() == "true");

            if !enable_debug_metrics && !enable_debug_cam {
                session
                    .app
                    .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
                        Duration::from_secs_f64(1.0 / 120.0),
                    )));
            } else {
                session
                    .app
                    .add_plugins(DefaultPlugins)
                    // .add_plugins(LogDiagnosticsPlugin::default())
                    .add_plugins(FrameTimeDiagnosticsPlugin)
                    .add_plugins(EntityCountDiagnosticsPlugin)
                    .add_plugins(SystemInformationDiagnosticsPlugin)
                    .add_plugins(PerfUiPlugin)
                    .add_systems(PostStartup, set_debug_metrics);

                if enable_debug_cam {
                    session
                        .app
                        .add_plugins(RapierDebugRenderPlugin::default())
                        .add_systems(PostStartup, set_debug_3d_render_camera)
                        .add_systems(Update, (move_debug_camera, look_debug_camera));
                } else {
                    session.app.add_systems(PostStartup, set_debug_metrics_cam);
                }
            }

            session
                .app
                .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
                .add_systems(Startup, (setup, setup_level).chain())
                .add_systems(PreUpdate, (handle_server_events).chain())
                .add_systems(
                    Update,
                    (
                        (
                            handle_character_movement,
                            handle_look_events,
                            handle_fire_events,
                            handle_hit_events,
                            handle_connect_events,
                            handle_disconnect_events,
                        )
                            .in_set(MySet::HandleGameEvents)
                            .after(MySet::HandleServer),
                        (on_transform_change, on_health_change)
                            .in_set(MySet::HandleGameStateChanges)
                            .after(MySet::HandleGameEvents),
                        transport_send_packets.after(MySet::HandleGameStateChanges),
                    ),
                );

            session.app.insert_resource(network_resource);
            session.app.run();
        }
    }

    pub fn send_event<E: Event>(&mut self, player_id: &String, event: E) {
        let player_session_map = self.player_session_map.lock().unwrap();

        if let Some(s_id) = player_session_map.get(player_id) {
            let mut sessions_lock = self.sessions.lock().unwrap();
            if let Some(session) = sessions_lock.get_mut(s_id) {
                session.app.world_mut().send_event(event);
            }
        }
    }

    pub fn join_session(&mut self, session_id: u64, player_id: String) {
        let mut player_session_map = self.player_session_map.lock().unwrap();

        if let None = player_session_map.get_mut(&player_id) {
            let mut sessions_lock = self.sessions.lock().unwrap();
            if let Some(session) = sessions_lock.get_mut(&session_id) {
                player_session_map.insert(player_id.clone(), session_id);

                session
                    .app
                    .world_mut()
                    .send_event(ConnectEvent { player_id });
            }
        }
    }
}

pub struct SessionCreateInput {
    pub id: u64,
    pub players: Vec<(String, u8)>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, SystemSet)]
pub enum MySet {
    HandleGameEvents,
    HandleServer,
    HandleGameStateChanges,
}
