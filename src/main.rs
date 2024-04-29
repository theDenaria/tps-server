use std::{io, sync::Arc};
mod event_in;
mod event_out;
mod game_state;
mod packet;
mod server;

use server::Server;

#[tokio::main]
async fn main() -> io::Result<()> {
    let server = Server::new(None).await; // Create server instance
    let server_arc = Arc::new(server); // Wrap the server instance in an Arc
    server_arc.start().await; // Call start on the Arc-wrapped instance
    Ok(())
}
