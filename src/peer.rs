use std::thread::JoinHandle;
use std::time::Duration;
use std::collections::{VecDeque, HashSet};
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::sync::mpsc::Receiver;
use std::iter::FromIterator;

use rand::Rng;
use rand::seq::SliceRandom;

use crate::message::{Message, MessageType};
use std::hash::{Hash, Hasher};
use crate::monitor::MonitoringConfig;

/// The peer sampling parameters
///
/// See: https://infoscience.epfl.ch/record/109297/files/all.pdf
#[derive(Clone)]
pub struct Config {
    /// Bind address for listening to incoming connections
    bind_address: String,
    /// Does the node push its view to other peers
    push: bool,
    /// When active, if the node will pull views from other peers
    /// When passive, if it responds with its view to pull from other peers
    pull: bool,
    /// The interval between each cycle of push/pull
    sampling_period: u64,
    /// Maximum value of random deviation from sampling interval
    sampling_deviation: u64,
    /// The number of peers in the node's view
    view_size: usize,
    /// The number of removal at each cycle
    healing_factor: usize,
    /// The number of peer swapped at each cycle
    swapping_factor: usize,
    /// Monitoring configuration
    monitoring: MonitoringConfig,
}

impl Config {
    /// Returns a configuration with specified parameters
    pub fn new(bind_address: String, push: bool, pull: bool, sampling_period: u64, sampling_deviation: u64, view_size: usize, healing_factor: usize, swapping_factor: usize, monitoring_config: Option<MonitoringConfig>) -> Config {
        let monitoring = match monitoring_config {
            Some(config) => config,
            None => MonitoringConfig::default(),
        };
        Config {
            bind_address,
            push,
            pull,
            sampling_period,
            sampling_deviation,
            view_size,
            healing_factor,
            swapping_factor,
            monitoring
        }
    }
}

/// The view at each node
struct View {
    /// The address of the node
    address: String,
    /// The list of peers the node is aware of
    peers: Vec<Peer>,
    /// The queue from which peer are retrieved for the application layer
    queue: VecDeque<Peer>,
}
impl View {
    /// Creates a new view with the node's address
    ///
    /// # Arguments
    ///
    /// * `address` - Addres of peer
    fn new(address: String) -> View {
        View {
            address,
            peers: vec![],
            queue: VecDeque::new(),
        }
    }

    /// Randomly select a peer for exchanging views at each cycle
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

    /// Move the oldest peers to the end of the view if the size
    /// of the view is larger than the argument
    ///
    /// # Arguments
    ///
    /// * `h` - The number of peer that should be moved
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

    /// Returns the peers at the beginning of the view
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    fn head(&self, c: usize) -> Vec<Peer> {
        let count = std::cmp::min(c / 2 - 1, self.peers.len());
        let mut head = Vec::new();
        for i in 0..count {
            head.push(self.peers[i].clone());
        }
        head
    }

    /// Increases by one the age of each peer in the view
    fn increase_age(&mut self) {
        for peer in self.peers.iter_mut() {
            peer.age += 1;
        }
    }

    /// Merge a view received received from a peer with the current view
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    /// * `h` - The healing parameter
    /// * `s` - The swap parameter
    /// * `buffer` - The view received
    fn select(&mut self, c:usize, h: usize, s: usize, buffer: &Vec<Peer>, monitoring_config: MonitoringConfig) {
        let my_address = self.address.clone();
        // Add received peers to current view, omitting the node's own address
        buffer.iter()
            .filter(|peer| peer.address != my_address)
            .for_each(|peer| self.peers.push(peer.clone()));
        self.remove_duplicates();
        self.remove_old_items(c, h);
        self.remove_head(c, s);
        self.remove_at_random(c);

        // Debug and monitoring
        let new_view = self.peers.iter()
            .map(|peer| peer.address.to_owned())
            .collect::<Vec<String>>();
        log::debug!("{}", new_view.join(", "));
        if monitoring_config.enabled() {
            monitoring_config.send_data(&self.address, new_view);
        }
    }

    /// Removes duplicates peers from the view by keeping the most recent one
    fn remove_duplicates(&mut self) {
        let mut unique_peers: HashSet<Peer> = HashSet::new();
        self.peers.iter().for_each(|peer| {
            if let Some(entry) = unique_peers.get(peer) {
                // duplicate peer, check age
                if peer.age < entry.age {
                    unique_peers.replace(peer.clone());
                }
            }
            else {
                // unique peer
                unique_peers.insert(peer.clone());
            }
        });
        let new_view = Vec::from_iter(unique_peers);
        std::mem::replace(&mut self.peers, new_view);
    }

    /// Removes the oldest items from the view based on the healing parameter
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    /// * `h` - The healing parameter
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
            std::mem::replace(&mut self.peers, new_view);
        }
    }

    /// Removes peers at the beginning of the current view based on the swap parameter
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    /// * `s` - The swap parameter
    fn remove_head(&mut self, c: usize, s: usize) {
        let min = if self.peers.len() > c { self.peers.len() - c } else { 0 };
        let removal_count = std::cmp::min(s, min);
        self.peers.drain(0..removal_count);
    }

    /// Removes peers at random to match the view size parameter
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    fn remove_at_random(&mut self, c: usize) {
        if self.peers.len() > c {
            for _ in 0..(self.peers.len() - c) {
                let remove_index = rand::thread_rng().gen_range(0, self.peers.len());
                self.peers.remove(remove_index);
            }
        }
    }

    /// Returns a random peer for use in the application layer
    /// The peer is selected from the queue of fresh peer if  available
    pub fn get_peer(&mut self) -> Option<Peer> {
        if let Some(peer) = self.queue.pop_front() {
            Some(peer)
        }
        else {
            self.select_peer()
        }
    }
}

// Byte separator between the peer address and the peer age
const SEPARATOR: u8 = 0x2C; // b','

/// Information about a peer
#[derive(Clone, Debug)]
pub struct Peer {
    /// Network address of the peer
    address: String,
    /// Age of the peer
    age: u16,
}

impl Peer {
    /// Creates a new peer with the specified address and age 0
    ///
    /// # Arguments
    ///
    /// * `address` - Network address of peer
    pub fn new(address: String) -> Peer {
        Peer {address, age: 0}
    }

    /// Increments the age of peer by one
    pub fn increment_age(&mut self) {
        if self.age < u16::max_value() {
            self.age += 1;
        }
    }

    /// Returns the age of peer
    pub fn age(&self) -> u16 {
        self.age
    }

    /// Returns the address of peer
    pub fn address(&self) -> &str { &self.address }

    /// Serializes peer into an array of bytes.
    /// Starts with the address of the peer first followed by the age of the peer
    /// Address and age are separated by a [SEPARATOR] byte.
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

    /// Deserializes a peer from an array of bytes
    ///
    /// # Arguments
    ///
    /// * `bytes` - A peer serialized as bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Peer, Box<dyn Error>> {
        // retrieve index of separator
        let separator_index = bytes.iter().enumerate()
            .find(|(_, b)| { **b == SEPARATOR})
            .map(|(i, _)| {i});
        if let Some(index) = separator_index {
            // check that there are exactly two bytes for the age after separator
            if bytes.len() != index + 3 {
                Err("invalid age")?
            }
            // retrieve address
            let address = String::from_utf8(bytes[..index].to_vec())?;
            // build age
            let age = ((bytes[index+1] as u16) << 8 ) + (bytes[index+2] as u16);
            Ok(Peer{
                address,
                age,
            })
        }
        else {
            Err("peer separator not found")?
        }
    }
}
impl Eq for Peer {}
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

/// Peer sampling service to by used by application
pub struct PeerSamplingService {
    /// Protocol parameters
    config: Config,
    /// View containing a list of other peers
    view: Arc<Mutex<View>>,
}

impl PeerSamplingService {

    /// Create a new peer sampling service with provided parameters
    ///
    /// # Arguments
    ///
    /// * `config` - The parameters for the peer sampling protocol
    pub fn new(config: Config) -> PeerSamplingService {
        PeerSamplingService {
            view: Arc::new(Mutex::new(View::new(config.bind_address.clone()))),
            config,
        }
    }

    /// Initializes service
    ///
    /// # Arguments
    ///
    /// * `initial_peer` - A closure returning the initial peer for starting the protocol
    pub fn init(&mut self, initial_peer: Box<dyn FnOnce() -> Option<Peer>>) -> JoinHandle<()> {
        // get address of initial peer
        if let Some(initial_peer) = initial_peer() {
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

    /// Returns a random peer for the client application
    /// The peer is pseudo-random peer from the set of all peers
    /// The local view is built using [Gossip-Based Peer Sampling]
    pub fn get_peer(&mut self) -> Option<&Peer> {
        unimplemented!()
    }

    /// Builds the view to be exchanged with another peer
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration parameters
    /// * `view` - The current view
    fn build_buffer(config: &Config, view: &mut View) -> Vec<Peer> {
        let mut buffer = vec![ Peer::new(config.bind_address.clone()) ];
        view.permute();
        view.move_oldest_to_end(config.healing_factor);
        buffer.append(&mut view.head(config.view_size));
        buffer
    }

    /// Creates a thread for handling messages
    ///
    /// # Arguments
    ///
    /// * `receiver` - The channel used for receiving incoming messages
    fn start_receiver(&self, receiver: Receiver<Message>) -> JoinHandle<()> {
        let config = self.config.clone();
        let view_arc = self.view.clone();
        std::thread::Builder::new().name(format!("{} - msg rx", config.bind_address)).spawn(move|| {
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
                    view.select(config.view_size, config.healing_factor, config.swapping_factor, &buffer, config.monitoring.clone());
                }
                else {
                    log::warn!("received a response with an empty buffer");
                }

                view.increase_age();
            }
        }).unwrap()
    }

    /// Creates a thread that periodically executes the peer sampling
    fn start_sampling_activity(&self) -> JoinHandle<()> {
        let config = self.config.clone();
        let view_arc = self.view.clone();
        std::thread::Builder::new().name(format!("{} - sampling", config.bind_address)).spawn(move || {
            loop {
                let deviation =
                    if config.sampling_deviation == 0 { 0 }
                    else { rand::thread_rng().gen_range(0, config.sampling_deviation * 1000) };
                let sleep_time = config.sampling_period * 1000 + deviation;
                std::thread::sleep(Duration::from_millis(sleep_time));
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
