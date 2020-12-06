mod config;
mod monitor;
mod message;
mod network;
mod peer;

pub use crate::config::Config;
pub use crate::peer::Peer;
pub use crate::peer::PeerSamplingService;

#[cfg(test)]
mod tests {
    use crate::{Peer, Config, PeerSamplingService};

    #[test]
    fn start_nodes() {
        // algorithm parameters
        let push = true;
        let pull = true;
        let t = 5;
        let d = 5;
        let c = 4;
        let h = 1;
        let s = 2;

        // create first peer with no contact peer
        let init_address = "127.0.0.1:9000";
        // configuration
        let first_config = Config::new(init_address.parse().unwrap(), push, pull, t, d, c, h, s, None);
        // no contact peer for first node
        let no_peer_handler = Box::new(move|| { None });

        // create and initiate the peer sampling service
        let mut join_handles = PeerSamplingService::new(first_config).init(no_peer_handler);

        // create peers using IPv4 addresses
        let mut port = 9001;
        for _ in 1..30 {
            // peer socket address
            let address = format!("127.0.0.1:{}", port);
            // configuration
            let config = Config::new(address.parse().unwrap(), push, pull, t, d, c, h, s, None);
            // closure for retrieving the address of the first contact peer
            let init_handler = Box::new(move|| { Some(Peer::new(init_address.to_owned())) });

            // create and initiate the peer sampling service
            let _handles = PeerSamplingService::new(config).init(init_handler);
            port += 1;
        }

        // create peers using IPv6 addresses
        for _ in 1..30 {
            // peer socket address
            let address = format!("[::1]:{}", port);
            // configuration
            let config = Config::new(address.parse().unwrap(), push, pull, t, d, c, h, s, None);
            // closure for retrieving the address of the first contact peer
            let init_handler = Box::new(move|| { Some(Peer::new(init_address.to_owned())) });

            // create and initiate the peer sampling service
            let _handles = PeerSamplingService::new(config).init(init_handler);
            port += 1;
        }

        join_handles.remove(0).join().unwrap();
    }
}
