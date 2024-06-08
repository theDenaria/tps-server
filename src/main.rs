#![allow(dead_code)]
use std::io;
mod constants;
mod ecs;
mod game_state;
mod server;

use ecs::systems::{
    handle_events::{
        handle_connect_events, handle_disconnect_events, handle_fire_events, handle_jump_events,
        handle_look_events, handle_move_events, HandleGameEvents,
    },
    handle_server::{
        handle_server_events, handle_server_messages, transport_send_packets, HandleServer,
    },
    on_change::{on_health_change, on_position_change, on_rotation_change, HandleGameStateChanges},
    physics::{handle_air_movement, physics_step, update_physic_components, Physics},
    setup::{setup, setup_level},
};
use server::transport::error::TransportError;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use bevy_ecs::prelude::*;

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Now the tracing macros can be used throughout your application
    tracing::info!("This will dynamically update on the terminal");

    let _ = start_server();

    Ok(())
}

fn start_server() -> Result<(), TransportError> {
    let mut world = World::default();

    let mut setup_schedule = Schedule::default();

    setup_schedule.add_systems((setup, setup_level).chain());
    setup_schedule.run(&mut world);

    let mut schedule = Schedule::default();

    schedule.add_systems((
        (handle_server_messages, handle_server_events)
            .chain()
            .in_set(HandleServer),
        (
            handle_move_events,
            handle_look_events,
            handle_fire_events,
            handle_jump_events,
            handle_connect_events,
            handle_disconnect_events,
        )
            .in_set(HandleGameEvents)
            .after(HandleServer),
        (physics_step, handle_air_movement, update_physic_components)
            .chain()
            .in_set(Physics)
            .after(HandleGameEvents),
        (on_position_change, on_rotation_change, on_health_change)
            .in_set(HandleGameStateChanges)
            .after(Physics),
        transport_send_packets.after(HandleGameStateChanges),
    ));

    // Your gameplay loop
    loop {
        // Receive new messages and update clients
        schedule.run(&mut world);
    }
}
