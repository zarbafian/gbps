use std::thread::JoinHandle;
use std::time::Duration;
use std::collections::{VecDeque, HashSet};
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::sync::mpsc::Receiver;

use rand::Rng;
use rand::seq::SliceRandom;

use crate::message::{Message, MessageType};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct Config {

    icon: char,
    bind_address: String,

    push: bool,
    pull: bool,
    sampling_period: u64,
    view_size: usize,
    healing_factor: usize,
    swapping_factor: usize,
}

impl Config {
    pub fn new(icon: char, bind_address: String, push: bool, pull: bool, sampling_period: u64, view_size: usize, healing_factor: usize, swapping_factor: usize) -> Config {
        Config {
            icon,
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
    address: String,
    peers: Vec<Peer>,
    queue: VecDeque<Peer>,
}
impl View {
    fn new(address: String) -> View {
        View {
            address,
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
        let count = std::cmp::min(c / 2 - 1, self.peers.len());
        let mut head = Vec::new();
        for i in 0..count {
            head.push(self.peers[i].clone());
        }
        head
    }
    fn increase_age(&mut self) {
        for peer in self.peers.iter_mut() {
            peer.age += 1;
        }
    }
    fn select(&mut self, c:usize, h: usize, s: usize, buffer: &Vec<Peer>) {
        let my_address = self.address.clone();
        log::debug!("my initial peers: {:?}", self.peers);
        buffer.iter().filter(|peer| peer.address != my_address).for_each(|peer| self.peers.push(peer.clone()));
        self.remove_duplicates();
        self.remove_old_items(c, h);
        self.remove_head(c, s);
        self.remove_at_random(c);

        // TODO
        crate::debug::print_peers(&self.peers);
    }

    fn remove_duplicates(&mut self) {
        let mut unique_peers: HashSet<Peer> = HashSet::new();
        self.peers.iter().for_each(|peer| {
            if let Some(entry) = unique_peers.get(peer) {
                if peer.age < entry.age {
                    unique_peers.replace(peer.clone());
                }
            }
            else {
                unique_peers.insert(peer.clone());
            }
        });
        self.peers.clear();
        for peer in unique_peers {
            self.peers.push(peer)
        };
    }
    fn remove_old_items(&mut self, c: usize, h: usize) {
        let min = if self.peers.len() > c { self.peers.len() - c } else { 0 };
        let removal_count = std::cmp::min(h, min);
        if removal_count > 0 {
            let mut sorted_by_age = self.peers.clone();
            sorted_by_age.sort_by_key(|peer| peer.age);
            sorted_by_age.truncate(sorted_by_age.len() - removal_count);
            let mut new_view = vec![];
            for peer in &self.peers {
                if sorted_by_age.contains(&peer) {
                    new_view.push(peer.clone());
                }
            }
            self.peers = new_view;
        }
    }
    fn remove_head(&mut self, c: usize, s: usize) {
        let min = if self.peers.len() > c { self.peers.len() - c } else { 0 };
        let removal_count = std::cmp::min(s, min);
        if removal_count > 0 {
            for _ in 0..removal_count {
                self.peers.remove(0);
            }
        }
    }
    fn remove_at_random(&mut self, c: usize) {
        if self.peers.len() > c {
            for _ in 0..(self.peers.len() - c) {
                let remove_index = rand::thread_rng().gen_range(0, self.peers.len());
                self.peers.remove(remove_index);
            }
        }
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
        v.push((self.age & 0x00FF) as u8);
        v
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Peer, Box<dyn Error>> {
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
            let age = ((bytes[index+1] as u16) << 8 ) + (bytes[index+2] as u16);
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
impl Eq for Peer {

}
impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}
impl Hash for Peer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.hash(state)
    }
}

pub struct PeerSamplingService {
    config: Config,
    view: Arc<Mutex<View>>,
}

impl PeerSamplingService {

    pub fn new(config: Config) -> PeerSamplingService {
        let view_address = config.bind_address.clone();
        PeerSamplingService {
            view: Arc::new(Mutex::new(View::new(view_address))),
            config,
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
        unimplemented!()
    }

    fn build_buffer(config: &Config, view: &mut View) -> Vec<Peer> {
        let mut buffer = vec![ Peer::new(config.bind_address.clone()) ];
        view.permute();
        view.move_oldest_to_end(config.healing_factor);
        buffer.append(&mut view.head(config.view_size));
        buffer
    }
    fn start_receiver(&self, receiver: Receiver<Message>) -> JoinHandle<()> {
        let config = self.config.clone();
        let view_arc = self.view.clone();
        std::thread::Builder::new().name(format!("{} {}  - msg rx", config.icon, config.bind_address)).spawn(move|| {
            while let Ok(message) = receiver.recv() {
                log::debug!("Received: {:?}", message);
                let mut view = view_arc.lock().unwrap();
                if let MessageType::Request = message.message_type() {
                    if config.pull {
                        let buffer = Self::build_buffer(&config, &mut view);
                        log::debug!("built response buffer: {:?}", buffer);
                        match crate::network::send(&message.sender(), Message::new_response(config.bind_address.clone(), Some(buffer))) {
                            Ok(()) => log::debug!("Buffer sent successfully"),
                            Err(e) => log::error!("Error sending buffer: {}", e),
                        }
                    }
                }

                if let Some(buffer) = message.view() {
                    view.select(config.view_size, config.healing_factor, config.swapping_factor, &buffer);
                }
                else {
                    log::warn!("received a response with an empty buffer");
                }

                view.increase_age();
            }
        }).unwrap()
    }

    fn start_sampling_activity(&self) -> JoinHandle<()> {
        let config = self.config.clone();
        let view_arc = self.view.clone();
        std::thread::Builder::new().name(format!("{} {} - sampling", config.icon, config.bind_address)).spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(config.sampling_period));
                log::debug!("Starting sampling protocol");
                let mut view = view_arc.lock().unwrap();
                if let Some(peer) = view.select_peer() {
                    if config.push {
                        let buffer = Self::build_buffer(&config, &mut view);
                        // send local view
                        match crate::network::send(&peer.address, Message::new_request(config.bind_address.clone(), Some(buffer))) {
                            Ok(()) => log::debug!("Buffer sent successfully"),
                            Err(e) => log::error!("Error sending buffer: {}", e),
                        }
                    }
                    else {
                        // send empty view to trigger response
                        match crate::network::send(&peer.address, Message::new_request(config.bind_address.clone(), None)) {
                            Ok(()) => log::debug!("Empty view sent successfully"),
                            Err(e) => log::error!("Error sending empty view: {}", e),
                        }
                    }
                    view.increase_age();
                }
                else {
                    log::warn!("No peer found for sampling")
                }
            }
        }).unwrap()
    }
}
