use crate::peer::Peer;
use std::net::TcpStream;
use std::error::Error;
use std::io::Write;

pub fn send(peer: Peer, buffer: Vec<Peer>) -> Result<(), Box<dyn Error>> {
    log::debug!("Sending -> {:?} to {:?}", buffer, peer);

    let mut stream = TcpStream::connect(peer.address())?;
        //.expect(&format!("Couldn't connect to the peer {}", peer.address()));
    let message = buffer.iter()
        .map(|peer| format!("{},{}", peer.address(), peer.age()))
        .collect::<Vec<String>>().join(" ");
    let written = stream.write(message.as_bytes())?;
    log::debug!("Written {} bytes", written);
    Ok(())
}