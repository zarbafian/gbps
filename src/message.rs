use crate::peer::Peer;
use std::error::Error;
use std::fmt::{Formatter, Debug};

const MSG_TYPE_REQ: u8 = 0x80; // 0b1000000
const MSG_TYPE_RESP: u8 = 0x00;

const MASK_MSG_TYPE: u8 = 0x80; // 0b1000000

//#[derive(Debug)]
pub enum MessageType {
    Request,
    Response
}
impl Debug for MessageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            MessageType::Request => "Request 🗞",
            MessageType::Response => "Response 📬",
        };
        write!(f, "{}", value)
    }
}

#[derive(Debug)]
pub struct Message {
    sender: String,
    message_type: MessageType,
    view: Option<Vec<Peer>>,
}

impl Message {

    pub fn new_request(sender: String, view: Option<Vec<Peer>>) -> Message {
        Self::new(sender, MessageType::Request, view)
    }

    pub fn new_response(sender: String, view: Option<Vec<Peer>>) -> Message {
        Self::new(sender, MessageType::Response, view)
    }

    fn new(sender: String, message_type: MessageType, view: Option<Vec<Peer>>) -> Message {
        Message{
            sender,
            message_type,
            view
        }
    }

    pub fn sender(&self) -> &str {
        &self.sender
    }
    pub fn message_type(&self) -> &MessageType{
        &self.message_type
    }

    pub fn view(&self) -> &Option<Vec<Peer>> {
        &self.view
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buffer = vec![];
        // first byte: message type
        match self.message_type {
            MessageType::Request => buffer.push(MSG_TYPE_REQ),
            MessageType::Response => buffer.push(MSG_TYPE_RESP),
        }
        // sender
        buffer.push(self.sender.as_bytes().len() as u8);
        self.sender.as_bytes().iter().for_each(|byte| buffer.push(*byte));
        // view
        if let Some(peers) = &self.view {
            // view size in number of peers
            buffer.push(peers.len() as u8);
            // rest of bytes: peers
            peers.iter().map(|p| {p.as_bytes()}).for_each(|mut bytes| {
                // length of peer data in bytes
                buffer.push(bytes.len() as u8);
                // peer data
                buffer.append(&mut bytes);
            });
        }
        else {
            // empty set
            buffer.push(0);
        }
        buffer
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Message, Box<dyn Error>> {
        // message type
        let message_type = match bytes[0] & MASK_MSG_TYPE {
            MSG_TYPE_REQ => MessageType::Request,
            MSG_TYPE_RESP => MessageType::Response,
            _ => return Err("invalid message type")?,
        };
        // sender
        let sender_size = bytes[1] as usize;
        let sender = String::from_utf8(bytes[2..2+sender_size].to_vec())?;
        //log::debug!("parsed sender is {}", sender);

        let view_size = bytes[2+sender_size];
        //log::debug!("parsed view_size is {}", view_size);
        if view_size > 0 {
            let mut index = 3+sender_size;
            let mut peers = vec![];
            for _ in 0..view_size {
                let peer_length = bytes[index] as usize;
                let parsed_peer = Peer::from_bytes(&bytes[index+1..index+1+peer_length])?;
                peers.push(parsed_peer);
                index += peer_length + 1;
            }
            //log::debug!("parsed peers is {:?}", peers);
            Ok(Message {
                sender,
                message_type,
                view: Some(peers)
            })
        }
        else {
            Ok(Message {
                sender,
                message_type,
                view: None
            })
        }
    }
}