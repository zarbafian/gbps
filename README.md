# Introduction
This crate is a Rust implementation of the algorithm for gossip-based peer sampling proposed by Jelasity et al [[1]]. 
It enables the creation of an overlay network among all the participating nodes and is meant for use for peer selection in gossip protocols.

# Peer Sampling
In large distributed systems that use gossip protocols for communication, peers should be selected at random in the network. In theory, this requires a knowledge of all the participating nodes. [Gossip-based Peer Sampling](https://infoscience.epfl.ch/record/109297/files/all.pdf) [[1]] is an algorithm by Jelasity et al that solves the random peer selection problem.

# Overview of the Algorithm
The algorithm consists of rounds of push/pull when peers exchange their views. During each round a node selects a peer at random and either push its view (if push is enabled) or an empty view to trigger a pull (if push is disabled). 
The selected node will process the view received, and possibly responds with its own view if pull is enabled.

When a node starts, it either connects to another node, so they can exchange their views, or does not know of any other node and wait for incoming push request (in the case of the initial node in a network).

Various strategies can be used to bootstrap the network (e.g. one initial node, multiple nodes, DNS service,...). This behavior can be defined in the closure parameter provided when starting a node. 

# API
The crate provides a `PeerSamplingService` that contains the two methods described in the article:
 - `init`: initializes the peer sampling protocol
 - `get_peer`: returns a peer at random for the gossip protocol 

It also has a `shutdown` method to terminate the different threads that were started for managing the peer sampling protocol.

# Configuration
The configuration parameters are the same as those presented in the paper:
 - `push`: push data
 - `pull`: pull data
 - `T`: duration of each cycle
 - `c`: size of local view
 - `H`: healing factor
 - `S`: swapping factor
 
Please refer to the article for the recommended values to use as parameters. In our tests we had enabled push and pull, selected values for `c` between 16 and 30, and had `c/2 = H + S`.

# Example
In the following code we start a first process with no contact peer, and a second process that knows only of the first process.

Starting the initial peer that does not know of any other node:
```
// configuration
let config = Config::new("127.0.0.1:9000".parse().unwrap(), true, true, 6, 5, 20, 2, 8, None);

// closure that returns no contact peer
let no_initial_peer = Box::new(move|| { None });

// create and initiate the peer sampling service
let mut sampling_service = PeerSamplingService::new(config);
sampling_service.init(no_initial_peer);

...
// std::thread::sleep(std::time::Duration::from_secs(20));

// terminate peer sampling
sampling_service.shutdown().unwrap();
```
Starting the second peer that will connect to the initial peer:
```
// configuration
let config = Config::new("127.0.0.1:9001".parse().unwrap(), true, true, 6, 5, 20, 2, 8, None);

// closure for retrieving the address of the initial contact peer
let initial_peer = Box::new(move|| { Some(Peer::new("127.0.0.1:9000".to_owned())) });

// create and initiate the peer sampling service
let mut sampling_service = PeerSamplingService::new(config);
sampling_service.init(initial_peer);

...
// std::thread::sleep(std::time::Duration::from_secs(20));

// terminate peer sampling
sampling_service.shutdown().unwrap();
```
Here is an example of what can be obtained with 61 local processes.

![alt text](https://github.com/pouriya-zarbafian/gbps/blob/master/assets/demo.png "Example with 61 local nodes")

[1]: https://infoscience.epfl.ch/record/109297/files/all.pdf
[[1]]: M. Jelasity, S. Voulgaris, R. Guerraoui, A.-M. Kermarrec, M. van Steen, Gossip-based Peer Sampling, 2007