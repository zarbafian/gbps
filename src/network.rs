use std::net::{TcpStream, TcpListener, SocketAddr};
use std::error::Error;
use std::io::{Read, Write};
use std::thread::JoinHandle;

use crate::peer::Peer;
use crate::message::Message;

pub fn start_listener(bind_address: &str) -> JoinHandle<()> {
    let listener = TcpListener::bind(bind_address).expect("error whith bind address");
    log::info!("Started listener on {}", bind_address);
    std::thread::Builder::new().name(bind_address.to_string()).spawn(move || {
        for incoming_stream in listener.incoming() {
            if let Ok(mut stream) = incoming_stream {
                let mut buf = Vec::new();
                if let Ok(count) = stream.read_to_end(&mut buf) {
                    let msg = Message::from_bytes(&*buf).expect("could not parse message");
                    //let msg = String::from_utf8(buf.clone()).unwrap();
                    log::debug!("Received: {:?}", msg);
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
    /*
    let message = message.view().iter()
        .map(|peer| format!("{},{}", peer.address(), peer.age()))
        .collect::<Vec<String>>().join(" ");
     */
    let written = stream.write(&message.as_bytes())?;
    log::debug!("Written {} bytes", written);
    Ok(())
}