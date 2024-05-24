use std::net::{TcpStream, TcpListener, SocketAddr};
use std::error::Error;
use std::io::{Read, Write};
use std::thread::JoinHandle;

use slog::{debug, error, info, warn, Logger};

use crate::message::Message;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Create a thread for listening to TCP connections
///
/// # Arguments
///
/// * `bind_address` - The socket bind address
/// * `sender` - A sender for notifying of received messages
pub fn start_listener(bind_address: &SocketAddr, sender: Sender<Message>, shutdown_handle: &Arc<AtomicBool>, logger: Logger) -> JoinHandle<()> {

    let listener = TcpListener::bind(bind_address)
        .expect(&format!("Could not listen to bind_address {}", bind_address));
    info!(logger, "Listening on {}", bind_address);

    // shutdown flag
    let shutdown_requested = Arc::clone(shutdown_handle);

    std::thread::Builder::new().name(format!("{} - gbps listener", bind_address)).spawn(move || {
        info!(logger, "Started listener thread");
        // TOD: handle hanging connections wher peer connect but does not write
        for incoming_stream in listener.incoming() {

            // check for shutdown request
            if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                info!(logger, "Shutdown requested");
                break;
            }

            // handle request
            match incoming_stream {
                Ok(mut stream) => {
                    if let Err(e) = handle_message(&mut stream, &sender, logger.clone()) {
                        error!(logger, "Error processing request: {}", e);
                    }
                }
                Err(e) => warn!(logger, "Connection failed: {}", e),
            }
        }
        info!(logger, "Listener thread exiting");
    }).unwrap()
}

fn handle_message(stream: &mut TcpStream, sender: &Sender<Message>, logger: Logger) -> Result<(), Box<dyn Error>>{
    debug!(logger, "handle_message");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let message = Message::from_bytes(&buf)?;
    sender.send(message)?;
    Ok(())
}

/// Sends a message to another peer
///
/// # Arguments
///
/// * `address` - Address of the peer
/// * `message` - The message to be sent
pub fn send(address: &SocketAddr, message: Message, logger: Logger) -> Result<(), Box<dyn Error>> {
    debug!(logger, "Sending -> {:?} to {:?}", message, address);
    let mut stream = TcpStream::connect(address)?;
    stream.write(&message.as_bytes())?;
    Ok(())
}