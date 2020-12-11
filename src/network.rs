use std::net::{TcpStream, TcpListener, SocketAddr};
use std::error::Error;
use std::io::{Read, Write};
use std::thread::JoinHandle;

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
pub fn start_listener(bind_address: &SocketAddr, sender: Sender<Message>, shutdown_handle: &Arc<AtomicBool>) -> JoinHandle<()> {

    let listener = TcpListener::bind(bind_address)
        .expect(&format!("Could not listen to bind_address {}", bind_address));
    log::info!("Listening on {}", bind_address);

    // shutdown flag
    let shutdown_requested = Arc::clone(shutdown_handle);

    std::thread::Builder::new().name(format!("{} - gbps listener", bind_address)).spawn(move || {
        log::info!("Started listener thread");
        // TOD: handle hanging connections wher peer connect but does not write
        for incoming_stream in listener.incoming() {

            // check for shutdown request
            if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                log::info!("Shutdown requested");
                break;
            }

            // handle request
            match incoming_stream {
                Ok(mut stream) => {
                    if let Err(e) = handle_message(&mut stream, &sender) {
                        log::error!("Error processing request: {}", e);
                    }
                }
                Err(e) => log::warn!("Connection failed: {}", e),
            }
        }
        log::info!("Listener thread exiting");
    }).unwrap()
}

fn handle_message(stream: &mut TcpStream, sender: &Sender<Message>) -> Result<(), Box<dyn Error>>{
    log::debug!("handle_message");
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
pub fn send(address: &SocketAddr, message: Message) -> Result<(), Box<dyn Error>> {
    log::debug!("Sending -> {:?} to {:?}", message, address);
    let mut stream = TcpStream::connect(address)?;
    stream.write(&message.as_bytes())?;
    Ok(())
}