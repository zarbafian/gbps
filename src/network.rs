use crate::peer::Peer;

pub fn send(peer: Peer, buffer: Vec<Peer>) {
    log::debug!("sending -> {:?}", buffer);
}