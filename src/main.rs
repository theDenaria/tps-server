use std::{io, sync::Arc};
mod connection_handler;
mod event_in;
mod event_out;
mod game_state;
mod packet;
mod server;

use server::Server;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Now the tracing macros can be used throughout your application
    tracing::info!("This will dynamically update on the terminal");

    let server = Server::new(None).await; // Create server instance
    let server_arc = Arc::new(server); // Wrap the server instance in an Arc
    server_arc.start().await; // Call start on the Arc-wrapped instance
    Ok(())
}
