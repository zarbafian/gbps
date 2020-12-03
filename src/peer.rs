use std::thread::JoinHandle;
use std::time::Duration;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream};
use std::io::Read;

use rand::Rng;
use rand::seq::SliceRandom;
use crate::message::Message;
use std::error::Error;
use std::ops::Index;
use std::sync::mpsc::Receiver;

#[derive(Clone)]
pub struct Config {

    bind_address: String,

    push: bool,
    pull: bool,
    sampling_period: u64,
    view_size: usize,
    healing_factor: usize,
    swapping_factor: usize,
}

impl Config {
    pub fn new(bind_address: String, push: bool, pull: bool, sampling_period: u64, view_size: usize, healing_factor: usize, swapping_factor: usize) -> Config {
        Config {
            bind_address,
            push,
            pull,
            sampling_period,
            view_size,
            healing_factor,
            swapping_factor
        }
    }
}

struct View {
    peers: Vec<Peer>,
    queue: VecDeque<Peer>,
}
impl View {
    fn new() -> View {
        View {
            peers: vec![],
            queue: VecDeque::new(),
        }
    }
    fn select_peer(&self) -> Option<Peer> {
        if self.peers.is_empty() {
            None
        }
        else {
            let selected_peer = rand::thread_rng().gen_range(0, self.peers.len());
            Some(self.peers[selected_peer].clone())
        }
    }

    /// Randomly reorder the current view
    fn permute(&mut self) {
        self.peers.shuffle(&mut rand::thread_rng());
    }

    fn move_oldest_to_end(&mut self, h: usize) {
        if self.peers.len() > h {
            let mut sorted_by_age = self.peers.clone();
            sorted_by_age.sort_by_key(|peer| peer.age);
            sorted_by_age.reverse();
            sorted_by_age.truncate(h);
            // (peers.len - h) at th beginning, h at the end
            let mut view_start = vec![];
            let mut view_end = vec![];
            for peer in &self.peers {
                if sorted_by_age.contains(&peer) {
                    view_end.push(peer.clone());
                }
                else {
                    view_start.push(peer.clone());
                }
            }
            view_start.append(&mut view_end);
            self.peers = view_start;
        }
    }
    fn head(&self, c: usize) -> Vec<Peer> {
        let count = std::cmp::min(c / 2 - 1, self.peers.len() - 1);
        let mut head = Vec::new();
        for i in 0..count {
            head.push(self.peers[i].clone());
        }
        head
    }
    fn select(&self, c:usize, h: usize, s: usize, buffer: Vec<Peer>) {
        // TODO
    }

    pub fn get_peer(&mut self) -> Option<Peer> {
        if let Some(peer) = self.queue.pop_front() {
            Some(peer)
        }
        else {
            self.select_peer()
        }
    }
}

const SEPARATOR: u8 = 0x2C; // b','

#[derive(Clone, Debug)]
pub struct Peer {
    address: String,
    age: u16,
}

impl Peer {
    pub fn new(address: String) -> Peer {
        Peer {address, age: 0}
    }
    pub fn increment_age(&mut self) {
        self.age += 1;
    }
    pub fn age(&self) -> u16 {
        self.age
    }
    pub fn address(&self) -> &str { &self.address }
    pub fn as_bytes(&self) -> Vec<u8> {
        // peer address
        let mut v = self.address.as_bytes().to_vec();
        // separator
        v.push(SEPARATOR);
        // peer age: first byte
        v.push((self.age >> 8) as u8);
        // peer age: second byte
        v.push((self.age & 0xff) as u8);
        v
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Peer, Box<Error>> {
        // retrieve index of separator
        let separator_index = bytes.iter().enumerate()
            .find(|(_, b)| { **b == SEPARATOR})
            .map(|(i, _)| {i});
        if let Some(index) = separator_index {
            // check that there are two bytes for the age after the separator
            if bytes.len() != index + 3 {
                Err("invalid age")?
            }
            let address = String::from_utf8(bytes[..index].to_vec())?;
            let age = (bytes[index+1] as u16) << 8 + bytes[index+2] as u16;
            Ok(Peer{
                address,
                age,
            })
        }
        else {
            Err("address separator not found")?
        }
    }
}
impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}
pub struct PeerSamplingService {
    config: Config,
    view: Arc<Mutex<View>>,
}

//type InitHandler = Box<dyn FnOnce() + Send + 'static>;

impl PeerSamplingService {

    pub fn new(config: Config) -> PeerSamplingService {
        PeerSamplingService {
            config,
            view: Arc::new(Mutex::new(View::new())),
        }
    }

    pub fn init(&mut self, init: Box<dyn FnOnce() -> Option<Peer>>) -> JoinHandle<()> {
        // get address of initial peer(s)
        if let Some(initial_peer) = init() {
            self.view.lock().unwrap().peers.push(initial_peer);
        }

        // listen to incoming message
        let (tx, rx) = std::sync::mpsc::channel();
        let listener_handler = crate::network::start_listener(&self.config.bind_address, tx);
        let receiver_handler = self.start_receiver(rx);

        // start peer sampling
        let sampling_handler = self.start_sampling_activity();

        // join threads
        listener_handler
        //sampling_handler.join().unwrap();
    }

    pub fn get_peer(&mut self) -> Option<&Peer> {
        None
    }

    fn start_receiver(&self, receiver: Receiver<Message>) -> JoinHandle<()> {
        std::thread::Builder::new().name("message receiver".to_string()).spawn(move|| {
            loop {
                if let Ok(message) = receiver.recv() {
                    log::debug!("Received: {:?}", message);
                }
            }
        }).unwrap()
    }

    fn start_sampling_activity(&self) -> JoinHandle<()> {
        let config = self.config.clone();
        let arc = self.view.clone();
        std::thread::Builder::new().name(config.bind_address.clone()).spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(config.sampling_period));
                log::debug!("Starting sampling protocol");
                let mut view = arc.lock().unwrap();
                if let Some(peer) = view.select_peer() {
                    if config.push {
                        let mut buffer = vec![ Peer::new(config.bind_address.clone()) ];
                        view.permute();
                        view.move_oldest_to_end(config.healing_factor);
                        buffer.append(&mut view.head(config.view_size));
                        match crate::network::send(&peer.address, Message::new_push(Some(buffer))) {
                            Ok(()) => log::debug!("Buffer sent successfully"),
                            Err(e) => log::error!("Error sending buffer: {}", e),
                        }
                    }
                    else {
                        // send empty view to trigger response
                        match crate::network::send(&peer.address, Message::new_push(None)) {
                            Ok(()) => log::debug!("Empty view sent successfully"),
                            Err(e) => log::error!("Error sending empty view: {}", e),
                        }
                    }
                }
                else {
                    log::warn!("No peer found for sampling")
                }
            }
        }).unwrap()
    }
}
