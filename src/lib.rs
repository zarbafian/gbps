mod message;
mod network;
mod conf;
mod peer;

#[cfg(test)]
mod tests {
    use crate::peer::{Config, PeerSamplingService, Peer};
    use crate::conf;
    use std::thread::JoinHandle;

    #[test]
    fn initial_peer() {
        conf::configure_logging("tmp/test.log".to_string(), "DEBUG".to_string()).unwrap();

        let alone_handle = start_node(
            Config::new("127.0.0.1:11111".to_string(),true, true, 3, 16, 1, 7),
            Box::new(|| { None })
        );

        let joining_handle_1 = start_node(
            Config::new("127.0.0.1:8888".to_string(),true, true, 3, 16, 1, 7),
            Box::new(|| { Some(Peer::new("127.0.0.1:11111".to_string())) })
        );

        let joining_handle_2 = start_node(
            Config::new("127.0.0.1:9999".to_string(),true, true, 3, 16, 1, 7),
            Box::new(|| { Some(Peer::new("127.0.0.1:11111".to_string())) })
        );

        alone_handle.join().unwrap()
    }

    fn start_node(config: Config, init_handler: Box<FnOnce() -> Option<Peer>>) -> JoinHandle<()> {
        let mut service = PeerSamplingService::new(config);
        service.init(init_handler)
    }
}
