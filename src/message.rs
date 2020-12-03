use crate::peer::Peer;
use std::error::Error;

const MSG_TYPE_PUSH: u8 = 0x80; // 0b1000000
const MSG_TYPE_PULL: u8 = 0x00;

const MASK_MSG_TYPE: u8 = 0x80; // 0b1000000

#[test]
fn bin_hexa() {
    println!("---------------------------------------------");
    println!("---------------------------------------------");
    println!("---------------------------------------------");
    println!("{:x}, {:x}", 0xabcd as u16 & 0xFF00, 0xabcd as u16 & 0x00FF);
    println!("{:x}, {:x}, {:x}", 0b1000_0000, 0b0100_0000, 0b0000_1111);
    println!("---------------------------------------------");
    println!("{:x}, {:x}, {:x}", 0xabcd, 0xabcd >> 8, 0xabcd & 0xff);
    println!("{:b}, {:x}", b',', b',');
    println!("---------------------------------------------");
    println!("{:x}, {:x}, {:x}", 0xabcd, 0xab << 8, (0xab << 8) + 0xcd);
    println!("---------------------------------------------");
}

#[derive(Debug)]
enum MessageType {
    Push, Pull
}
#[derive(Debug)]
pub struct Message {
    message_type: MessageType,
    view: Option<Vec<Peer>>,
}

impl Message {

    pub fn new_push(view: Option<Vec<Peer>>) -> Message {
        Self::new(MessageType::Push, view)
    }

    pub fn new_pull(view: Option<Vec<Peer>>) -> Message {
        Self::new(MessageType::Pull, view)
    }

    fn new(message_type: MessageType, view: Option<Vec<Peer>>) -> Message {
        Message{
            message_type,
            view
        }
    }

    pub fn view(&self) -> &Option<Vec<Peer>> {
        self.view()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buffer = vec![];
        // first byte: message type
        match self.message_type {
            MessageType::Push => buffer.push(MSG_TYPE_PUSH),
            MessageType::Pull => buffer.push(MSG_TYPE_PULL),
        }
        if let Some(peers) = &self.view {
            // second byte: view size in number of peers
            buffer.push(peers.len() as u8);
            // rest of bytes: view
            peers.iter().map(|p| {p.as_bytes()}).for_each(|mut bytes| {
                // length of peer data in bytes
                buffer.push(bytes.len() as u8);
                // peer data
                buffer.append(&mut bytes);
            });
        }
        buffer
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Message, Box<Error>> {
        let message_type = match bytes[0] & MASK_MSG_TYPE {
            MSG_TYPE_PUSH => MessageType::Push,
            MSG_TYPE_PULL => MessageType::Pull,
            _ => return Err("invalid message type")?,
        };
        let view_size = bytes[1];
        if view_size > 0 {
            let index = 2;
            let mut peers = vec![];
            for _ in 0..view_size {
                let peer_length = bytes[index] as usize;
                let parsed_peer = Peer::from_bytes(&bytes[index+1..index+1+peer_length])?;
                peers.push(parsed_peer);
            }
            Ok(Message {
                message_type,
                view: Some(peers)
            })
        }
        else {
            Ok(Message {
                message_type,
                view: None
            })
        }
    }
}