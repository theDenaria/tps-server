use std::collections::HashMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{channel::DefaultChannel, event_out::EventOut, server::MattaServer};

#[derive(Default, Component)]
pub struct Player {
    pub id: String,
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

#[derive(Default, Component, Serialize, Deserialize, Debug, Clone)]
pub struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Default, Component, Serialize, Deserialize, Debug, Clone)]
pub struct Rotation {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Default, Component)]
pub struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(Default, Component)]
pub struct Health(f32);

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    position: Position,
    rotation: Rotation,
    velocity: Velocity,
    health: Health,
}

impl Default for PlayerBundle {
    fn default() -> Self {
        PlayerBundle {
            player: Player { id: String::new() },
            position: Position::default(),
            rotation: Rotation::default(),
            velocity: Velocity::default(),
            health: Health::default(),
        }
    }
}

#[derive(Event)]
pub struct MoveEvent {
    entity: Entity,
    x: f32,
    y: f32,
}

#[derive(Event)]
pub struct LookEvent {
    entity: Entity,
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Event)]
pub struct JumpEvent {
    entity: Entity,
}

#[derive(Event)]
pub struct FireEvent {
    entity: Entity,
    position: Position,
    rotation: Rotation,
}

#[derive(Event)]
pub struct ConnectEvent {
    player_id: String,
}

#[derive(Event)]
pub struct DisconnectEvent {
    player_id: String,
}

static VELOCITY_MUL: f32 = 0.1;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HandleGameStateChanges;

// Gets the Position component of all Entities whose Velocity has changed since the last run of the System
pub fn on_position_change(
    mut query: Query<(&Player, &mut Position, &Velocity)>,
    mut server: ResMut<MattaServer>,
) {
    let mut positions: Vec<(Position, String)> = vec![];
    for (player, mut position, velocity) in query.iter_mut() {
        if !(velocity.x == 0.0 && velocity.x == 0.0 && velocity.x == 0.0) {
            if velocity.x != 0.0 {
                position.x = position.x + velocity.x;
            }
            if velocity.z != 0.0 {
                position.z = position.z + velocity.z;
            }
            if velocity.x != 0.0 {
                position.y = position.y + velocity.y;
            }
            positions.push((position.clone(), player.id.clone()));
        }
    }
    if let Some(position_event) = EventOut::position_event(positions) {
        server.broadcast_message(DefaultChannel::Unreliable, position_event.data);
    }
}

pub fn on_rotation_change(
    query: Query<(&Player, &Rotation), Changed<Rotation>>,
    mut server: ResMut<MattaServer>,
) {
    if let Some(rotation_event) = EventOut::rotation_event(&query) {
        tracing::trace!("ROTATION EVENT TO SEND: {:?}", rotation_event);
        server.broadcast_message(DefaultChannel::Unreliable, rotation_event.data);
    }
}

pub fn on_health_change(query: Query<(&Player, &Health), Changed<Health>>) {
    for (player, health) in &query {
        // Broadcast health update
    }
}

pub fn on_player_added(
    added_players_query: Query<(&Player, &Position, &Rotation, &Health), Added<Player>>,
    all_players_query: Query<(&Player, &Position, &Rotation)>,
    mut server: ResMut<MattaServer>,
) {
    for (player, position, rotation, health) in added_players_query.iter() {
        if let Ok(added_client_id) = server.client_id_by_player_id(player.id.clone()) {
            if let Some(spawn_all) = EventOut::spawn_event_for_all_players(&all_players_query) {
                server.send_message(
                    added_client_id,
                    DefaultChannel::ReliableOrdered,
                    spawn_all.data,
                )
            }
            let spawn =
                EventOut::spawn_new_event(player.id.clone(), position.clone(), rotation.clone());
            server.broadcast_message_except(
                added_client_id,
                DefaultChannel::ReliableOrdered,
                spawn.data,
            )
        }
    }
}

pub fn send_move_event(
    player_id: &String,
    move_x: f32,
    move_y: f32,
    player_lookup: &Res<PlayerLookup>,
    move_event: &mut EventWriter<MoveEvent>,
) {
    if let Some(player_entity) = player_lookup.map.get(player_id) {
        move_event.send(MoveEvent {
            entity: *player_entity,
            x: move_x,
            y: move_y,
        });
    } else {
        tracing::warn!("Player ID not found: {}", player_id);
    }
}

pub fn send_look_event(
    player_id: &String,
    look_x: f32,
    look_y: f32,
    look_z: f32,
    player_lookup: &Res<PlayerLookup>,
    look_event: &mut EventWriter<LookEvent>,
) {
    if let Some(player_entity) = player_lookup.map.get(player_id) {
        look_event.send(LookEvent {
            entity: *player_entity,
            x: look_x,
            y: look_y,
            z: look_z,
        });
    } else {
        tracing::warn!("Player ID not found: {}", player_id);
    }
}

pub fn send_jump_event(
    player_id: String,
    player_lookup: Res<PlayerLookup>,
    mut jump_event: EventWriter<JumpEvent>,
) {
    if let Some(player_entity) = player_lookup.map.get(&player_id) {
        jump_event.send(JumpEvent {
            entity: *player_entity,
        });
    } else {
        tracing::warn!("Player ID not found: {}", player_id);
    }
}

pub fn send_fire_event(
    player_id: String,
    position: Position,
    rotation: Rotation,
    player_lookup: Res<PlayerLookup>,
    mut fire_event: EventWriter<FireEvent>,
) {
    if let Some(player_entity) = player_lookup.map.get(&player_id) {
        fire_event.send(FireEvent {
            entity: *player_entity,
            position,
            rotation,
        });
    } else {
        tracing::warn!("Player ID not found: {}", player_id);
    }
}

pub fn send_connect_event(player_id: &String, connect_event: &mut EventWriter<ConnectEvent>) {
    connect_event.send(ConnectEvent {
        player_id: player_id.clone(),
    });
}

pub fn send_disconnect_event(
    player_id: String,
    disconnect_event: &mut EventWriter<DisconnectEvent>,
) {
    disconnect_event.send(DisconnectEvent { player_id });
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HandleGameEvents;

pub fn handle_move_events(
    mut move_events: EventReader<MoveEvent>,
    mut query: Query<&mut Velocity>,
) {
    for event in move_events.read() {
        if let Ok(mut velocity) = query.get_mut(event.entity) {
            velocity.x = event.x * VELOCITY_MUL;
            velocity.z = event.y * VELOCITY_MUL;
        }
    }
}

pub fn handle_look_events(
    mut look_events: EventReader<LookEvent>,
    mut query: Query<&mut Rotation>,
) {
    for event in look_events.read() {
        if let Ok(mut rotation) = query.get_mut(event.entity) {
            rotation.x = event.x;
            rotation.y = event.y;
            rotation.z = event.z;
        }
    }
}

pub fn handle_jump_events(
    mut jump_events: EventReader<LookEvent>,
    mut query: Query<&mut Position>,
) {
    for event in jump_events.read() {
        if let Ok(mut positon) = query.get_mut(event.entity) {
            //Handle jump
        }
    }
}

pub fn handle_fire_events(mut fire_events: EventReader<FireEvent>, mut query: Query<&mut Health>) {
    for event in fire_events.read() {}
}

pub fn handle_connect_events(
    mut commands: Commands,
    mut connect_events: EventReader<ConnectEvent>,
    mut player_lookup: ResMut<PlayerLookup>,
) {
    for event in connect_events.read() {
        if !player_lookup.map.contains_key(&event.player_id) {
            tracing::trace!("Handle connect event: {:?}", event.player_id);
            let entity = commands
                .spawn({
                    PlayerBundle {
                        player: Player {
                            id: event.player_id.clone(),
                        },
                        position: Position {
                            x: 10.0,
                            y: 6.0,
                            z: 5.0,
                        },
                        ..Default::default()
                    }
                })
                .id();
            player_lookup.map.insert(event.player_id.clone(), entity);
        }
    }
}

pub fn handle_disconnect_events(
    mut commands: Commands,
    mut disconnect_events: EventReader<DisconnectEvent>,
    mut player_lookup: ResMut<PlayerLookup>,
    mut server: ResMut<MattaServer>,
) {
    if disconnect_events.len() > 0 {
        let mut disconnect_player_ids: Vec<&String> = vec![];
        for event in disconnect_events.read() {
            if let Some(entity) = player_lookup.map.get(&event.player_id) {
                commands.entity(*entity).despawn();
                player_lookup.map.remove(&event.player_id);

                disconnect_player_ids.push(&event.player_id);
            }
        }
        let disconnect_event = EventOut::disconnect_event(disconnect_player_ids).unwrap();
        tracing::trace!("Disconnect event: {:?}", disconnect_event);
        server.broadcast_message(DefaultChannel::ReliableOrdered, disconnect_event.data);
    }
}
