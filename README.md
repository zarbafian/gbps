# Introduction
In large distributed systems that use gossip protocols for communication, peers should be selected at random in the network. In theory, this requires a knowledge of all the participating nodes. [Gossip-based Peer Sampling] [1] is an algorithm that solves the random peer selection problem.

This crate is a Rust implementation of the algorithm proposed by Jelasity et al [1]. It enables the creation of an overlay network for use in gossip protocols.

# API
The crate provides the two methods described in the article:
 - `init`: initializes the peer sampling protocol
 - `get_peer`: return a peer at random for a gossip protocol 

# Configuration
The configuration parameters are the same as those presented in the paper:
 - `push`: push data
 - `pull`: pull data
 - `T`: period of each cycle
 - `c`: size of local view
 - `H`: healing factor
 - `S`: swapping factor
 
Please refer to the article for the recommended values to use as parameters.
 
Each node also requires the address of a peer to start exchanging views, except for the first node that will wait for incoming connections.

The selection of the initial peer to contact can be customized using a closure.

# Example
In the following code we start a first process with no contact peer, and a second process that knows only of the first process.
```
// first peer
let first_address = "127.0.0.1:9000";

// configuration
let first_config = Config::new(first_address.parse().unwrap(), true, true, 60, 5, 20, 2, 8, None);

// closure that returns no contact peer
let no_contact = Box::new(move|| { None });

// create and initiate the peer sampling service
let mut handles = PeerSamplingService::new(first_config).init(no_contact);

// second peer
let second_address = "127.0.0.1:9001";

// configuration
let second_config = Config::new(second_address.parse().unwrap(), true, true, 60, 5, 20, 2, 8, None);

// closure for retrieving the address of the first contact peer
let init_handler = Box::new(move|| { Some(Peer::new(first_address.to_owned())) });

// create and initiate the peer sampling service
let mut handles = PeerSamplingService::new(second_config).init(init_handler);

// join on the created threads otherwise the program will terminate
// handles.remove(0).join();
```
Here is the result obtained with 61 local processes.
(https://github.com/pouriya-zarbafian/gbps/demo.png)

[1]: M. Jelasity, S. Voulgaris, R. Guerraoui, A.-M. Kermarrec, M. van Steen, Gossip-based Peer Sampling, 2007