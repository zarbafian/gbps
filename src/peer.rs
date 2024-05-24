use std::thread::JoinHandle;
use std::time::Duration;
use std::collections::{VecDeque, HashSet};
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::sync::mpsc::Receiver;
use std::iter::FromIterator;

use rand::Rng;
use rand::seq::SliceRandom;
use slog::{debug, error, info, warn, Logger};

use crate::message::{Message, MessageType};
use std::hash::{Hash, Hasher};
use crate::monitor::MonitoringConfig;
use crate::config::Config;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;

/// The view at each node
struct View {
    /// The address of the node
    host_address: String,
    /// The list of peers in the node view
    peers: Vec<Peer>,
    /// The queue from which peer are retrieved for the application layer
    queue: VecDeque<Peer>,
    /// Logger
    logger: Logger,
}
impl View {
    /// Creates a new view with the node's address
    ///
    /// # Arguments
    ///
    /// * `address` - Addres of peer
    /// * `logger` - Logger
    fn new(host_address: String, logger: Logger) -> View {
        View {
            host_address,
            peers: vec![],
            queue: VecDeque::new(),
            logger,
        }
    }

    /// Randomly select a peer for exchanging views at each cycle
    fn select_peer(&self) -> Option<Peer> {
        if self.peers.is_empty() {
            None
        }
        else {
            let selected_peer = rand::thread_rng().gen_range(0..self.peers.len());
            Some(self.peers[selected_peer].clone())
        }
    }

    /// Randomly reorder the current view
    fn permute(&mut self) {
        self.peers.shuffle(&mut rand::thread_rng());
    }

    /// Move the oldest peers to the end of the view if the size
    /// of the view is larger than the healing factor
    ///
    /// # Arguments
    ///
    /// * `h` - The number of peer that should be moved
    fn move_oldest_to_end(&mut self, h: usize) {
        if self.peers.len() > h {
            let mut h_oldest_peers = self.peers.clone();
            h_oldest_peers.sort_by_key(|peer| peer.age);
            h_oldest_peers.reverse();
            h_oldest_peers.truncate(h); //
            // (peers.len - h) at the beginning, h at the end
            let mut new_view_start = vec![];
            let mut new_view_end = vec![];
            for peer in &self.peers {
                if h_oldest_peers.contains(&peer) {
                    new_view_end.push(peer.clone());
                }
                else {
                    new_view_start.push(peer.clone());
                }
            }
            new_view_start.append(&mut new_view_end);
            let _ = std::mem::replace(&mut self.peers, new_view_start);
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
        let my_address = self.host_address.clone();
        // Add received peers to current view, omitting the node's own address
        buffer.iter()
            .filter(|peer| peer.address != my_address)
            .for_each(|peer| self.peers.push(peer.clone()));
        // Perform peer selection algorithm
        self.remove_duplicates();
        self.remove_old_items(c, h);
        self.remove_head(c, s);
        self.remove_at_random(c);
        // Update peer queue for application layer
        self.update_queue();

        // Debug and monitoring
        let new_view = self.peers.iter()
            .map(|peer| peer.address.to_owned())
            .collect::<Vec<String>>();
        debug!(self.logger, "{}", new_view.join(", "));
        if monitoring_config.enabled() {
            monitoring_config.send_data(&self.host_address, new_view);
        }
    }

    /// Removes duplicates peers from the view and keep the most recent one
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
        let _ = std::mem::replace(&mut self.peers, new_view);
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
            let mut kept_peers = self.peers.clone();
            kept_peers.sort_by_key(|peer| peer.age);
            kept_peers.truncate(kept_peers.len() - removal_count);
            let mut new_view = vec![];
            for peer in &self.peers {
                if kept_peers.contains(&peer) {
                    new_view.push(peer.clone());
                }
            }
            let _ = std::mem::replace(&mut self.peers, new_view);
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
                let remove_index = rand::thread_rng().gen_range(0..self.peers.len());
                self.peers.remove(remove_index);
            }
        }
    }

    /// Update peer queue by adding peers that appeared in the view
    /// and removing those that were removed.
    fn update_queue(&mut self) {

        // compute index of removed peers
        let removed_peers = self.queue.iter().enumerate()
            .filter(|(_, peer)| !self.peers.contains(peer))
            .map(|(index, _)| index)
            .collect::<Vec<usize>>();

        // compute new peers
        let added_peers = self.peers.iter()
            .filter(|peer| !self.queue.contains(peer))
            .map(|peer| peer.to_owned())
            .collect::<Vec<Peer>>();

        // removed old peers by descending index
        removed_peers.iter().rev().for_each(|index| { self.queue.remove(*index); });

        // add new peers
        for peer in added_peers {
            self.queue.push_back(peer);
        }
    }

    /// Returns a random peer for use in the application layer.
    /// The peer is selected from the queue of newly added peers if available,
    /// otherwise at random from the view.
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
    /// Socket address of the peer
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
    /// address and age are separated by a [SEPARATOR] byte.
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
    // Handles for activity threads
    thread_handles: Vec<JoinHandle<()>>,
    /// Handle for shutting down the TCP listener thread
    shutdown_tcp_listener: Arc<AtomicBool>,
    /// Handle for shutting down the peer sampling thread
    shutdown_peer_sampling: Arc<AtomicBool>,
    /// Logger
    logger: Logger,
}

impl PeerSamplingService {

    /// Create a new peer sampling service with provided parameters
    ///
    /// # Arguments
    ///
    /// * `config` - The parameters for the peer sampling protocol
    pub fn new(config: Config, logger: Logger) -> PeerSamplingService {
        PeerSamplingService {
            view: Arc::new(Mutex::new(View::new(config.address().to_string(), logger.clone()))),
            config,
            thread_handles: Vec::new(),
            shutdown_tcp_listener: Arc::new(AtomicBool::new(false)),
            shutdown_peer_sampling: Arc::new(AtomicBool::new(false)),
            logger,
        }
    }

    /// Initializes service
    ///
    /// # Arguments
    ///
    /// * `initial_peer` - A closure returning the initial peer for starting the protocol
    pub fn init(&mut self, initial_peer: Box<dyn FnOnce() -> Option<Vec<Peer>>>) {
        // get address of initial peer
        if let Some(mut initial_peers) = initial_peer() {
            self.view.lock().unwrap().peers.append(&mut initial_peers);
        }

        // listen to incoming message
        let (tx, rx) = std::sync::mpsc::channel();
        let listener_handle = crate::network::start_listener(&self.config.address(), tx, &self.shutdown_tcp_listener, self.logger.clone());
        self.thread_handles.push(listener_handle);

        // handle received messages
        let receiver_handle = self.start_receiver(rx);
        self.thread_handles.push(receiver_handle);

        // start peer sampling
        let sampling_handle = self.start_sampling_activity();
        self.thread_handles.push(sampling_handle);

        info!(self.logger, "All activity threads were started");
    }

    /// Returns a random peer for the client application.
    /// The peer is pseudo-random peer from the set of all peers.
    /// The local view is built using [Gossip-Based Peer Sampling].
    pub fn get_peer(&mut self) -> Option<Peer> {
        self.view.lock().unwrap().get_peer()
    }

    /// Stops the threads related to peer sampling activity
    pub fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        // request shutdown
        self.shutdown_peer_sampling.store(true, std::sync::atomic::Ordering::SeqCst);
        self.shutdown_tcp_listener.store(true, std::sync::atomic::Ordering::SeqCst);
        {
            let guard = self.view.lock().unwrap();
            crate::network::send(&guard.host_address.parse()?, Message::new_response(guard.host_address.to_owned(), None), self.logger.clone())?;
        }
        // wait for termination
        let handles = self.thread_handles.drain(..);
        let mut join_error = false;
        for handle in handles {
            if let Err(e) = handle.join() {
                error!(self.logger, "Error joining thread: {:?}", e);
                join_error = true;
            }
        }
        info!(self.logger, "All activity threads were stopped");
        if join_error {
            Err("An error occurred during thread joining")?
        }
        else {
            Ok(())
        }
    }

    /// Builds the view to be exchanged with another peer
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration parameters
    /// * `view` - The current view
    fn build_buffer(config: &Config, view: &mut View) -> Vec<Peer> {
        let mut buffer = vec![ Peer::new(config.address().to_string()) ];
        view.permute();
        view.move_oldest_to_end(config.healing_factor());
        buffer.append(&mut view.head(config.view_size()));
        buffer
    }

    /// Creates a thread for handling messages
    ///
    /// # Arguments
    ///
    /// * `receiver` - The channel used for receiving incoming messages
    fn start_receiver(&self, receiver: Receiver<Message>) -> JoinHandle<()>{
        let config = self.config.clone();
        let view_arc = self.view.clone();
        let logger = self.logger.clone();
        std::thread::Builder::new().name(format!("{} - gbps receiver", config.address())).spawn(move|| {
            info!(logger, "Started message handling thread");
            while let Ok(message) = receiver.recv() {
                debug!(logger, "Received: {:?}", message);
                let mut view = view_arc.lock().unwrap();
                if let MessageType::Request = message.message_type() {
                    if config.is_pull() {
                        let buffer = Self::build_buffer(&config, &mut view);
                        debug!(logger, "Built response buffer: {:?}", buffer);
                        if let Ok(remote_address) = message.sender().parse::<SocketAddr>() {
                            match crate::network::send(&remote_address, Message::new_response(config.address().to_string(), Some(buffer)), logger.clone()) {
                                Ok(()) => debug!(logger, "Buffer sent successfully"),
                                Err(e) => error!(logger, "Error sending buffer: {}", e),
                            }
                        }
                        else {
                            error!(logger, "Could not parse sender address {}", &message.sender());
                        }
                    }
                }

                if let Some(buffer) = message.view() {
                    view.select(config.view_size(), config.healing_factor(), config.swapping_factor(), &buffer, config.monitoring().clone());
                }
                else {
                    warn!(logger, "received a response with an empty buffer");
                }

                view.increase_age();
            }
            info!(logger, "Message handling thread exiting");
        }).unwrap()
    }

    /// Creates a thread that periodically executes the peer sampling
    fn start_sampling_activity(&self) -> JoinHandle<()> {
        let config = self.config.clone();
        let view_arc = self.view.clone();
        let shutdown_requested = Arc::clone(&self.shutdown_peer_sampling);
        let logger = self.logger.clone();
        std::thread::Builder::new().name(format!("{} - gbps sampling", config.address())).spawn(move || {
            info!(logger, "Started peer sampling thread");
            loop {
                // Compute time for sleep cycle
                let deviation =
                    if config.sampling_deviation() == 0 { 0 }
                    else { rand::thread_rng().gen_range(0..(config.sampling_deviation() * 1000)) };
                let sleep_time = config.sampling_period() * 1000 + deviation;
                std::thread::sleep(Duration::from_millis(sleep_time));

                debug!(logger, "Sampling peers");
                let mut view = view_arc.lock().unwrap();
                if let Some(peer) = view.select_peer() {
                    if config.is_push() {
                        let buffer = Self::build_buffer(&config, &mut view);
                        // send local view
                        if let Ok(remote_address) = &peer.address.parse::<SocketAddr>() {
                            match crate::network::send(remote_address, Message::new_request(config.address().to_string(), Some(buffer)), logger.clone()) {
                                Ok(()) => debug!(logger, "Buffer sent successfully"),
                                Err(e) => error!(logger, "Error sending buffer: {}", e),
                            }
                        }
                        else {
                            error!(logger, "Could not parse sender address {}", &peer.address);
                        }
                    }
                    else {
                        // send empty view to trigger response
                        if let Ok(remote_address) = &peer.address.parse::<SocketAddr>() {
                            match crate::network::send(remote_address, Message::new_request(config.address().to_string(), None), logger.clone()) {
                                Ok(()) => debug!(logger, "Empty view sent successfully"),
                                Err(e) => error!(logger, "Error sending empty view: {}", e),
                            }
                        }
                        else {
                            error!(logger, "Could not parse sender address {}", &peer.address);
                        }
                    }
                    view.increase_age();
                }
                else {
                    warn!(logger, "No peer found for sampling")
                }

                // check for shutdown request
                if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
            }

            info!(logger, "Peer sampling thread exiting");
        }).unwrap()
    }
}
