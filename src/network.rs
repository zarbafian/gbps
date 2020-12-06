use std::net::{TcpStream, TcpListener, SocketAddr};
use std::error::Error;
use std::io::{Read, Write};
use std::thread::JoinHandle;

use crate::message::Message;
use std::sync::mpsc::Sender;

/// Create a thread for listening to TCP connections
///
/// # Arguments
///
/// * `bind_address` - The socket bind address
/// * `sender` - A sender for notifying of received messages
pub fn start_listener(bind_address: &SocketAddr, sender: Sender<Message>) -> JoinHandle<()> {
    let listener = TcpListener::bind(bind_address).expect(&format!("Could not listen to bind_address {}", bind_address));
    log::info!("Started listener on {}", bind_address);
    std::thread::Builder::new().name(format!("{} - listener", bind_address)).spawn(move || {
        for incoming_stream in listener.incoming() {
            match incoming_stream {
                Ok(mut stream) => {
                    if let Err(e) =  handle_message(&mut stream, &sender) {
                        log::error!("Error processing request: {}", e);
                    }
                }
                Err(e) => log::warn!("Connection failed: {}", e),
            }
        }
    }).unwrap()
}

fn handle_message(stream: &mut TcpStream, sender: &Sender<Message>) -> Result<(), Box<dyn Error>>{
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