use std::net::{TcpStream, TcpListener, SocketAddr};
use std::error::Error;
use std::io::{Read, Write};
use std::thread::JoinHandle;

use crate::peer::Peer;
use crate::message::Message;
use std::sync::mpsc::Sender;

pub fn start_listener(bind_address: &str, sender: Sender<Message>) -> JoinHandle<()> {
    let listener = TcpListener::bind(bind_address).expect("error whith bind address");
    log::info!("Started listener on {}", bind_address);
    std::thread::Builder::new().name(format!("{} - listener", bind_address)).spawn(move || {
        for incoming_stream in listener.incoming() {
            if let Ok(mut stream) = incoming_stream {
                let mut buf = Vec::new();
                if let Ok(count) = stream.read_to_end(&mut buf) {
                    if let Ok(message) = Message::from_bytes(&buf) {
                        sender.send(message);
                    }
                }
            } else {
                log::error!("Error with incoming connection");
            }
        }
    }).unwrap()
}

pub fn send(address: &str, message: Message) -> Result<(), Box<dyn Error>> {
    log::debug!("Sending -> {:?} to {:?}", message, address);

    let mut stream = TcpStream::connect(address)?;
        //.expect(&format!("Couldn't connect to the peer {}", peer.address()));

    let written = stream.write(&message.as_bytes())?;
    //log::debug!("Written {} bytes", written);
    Ok(())
}