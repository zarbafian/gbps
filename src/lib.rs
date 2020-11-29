mod network;
mod conf;
mod peer;

#[cfg(test)]
mod tests {
    use crate::peer::{Config, PeerSamplingService, Peer};
    use crate::conf;

    #[test]
    fn initial_peer() {
        conf::configure_logging("tmp/test.log".to_string(), "DEBUG".to_string()).unwrap();
        let init_handler = || { None };
        let config = Config::new("127.0.0.1:11111".to_string(),true, true, 3, 16, 1, 7);
        let mut service = PeerSamplingService::new(config);
        let handle = service.init(Box::new(init_handler));

        let init_handler = || { Some(Peer::new("127.0.0.1:11111".to_string())) };
        let config = Config::new("127.0.0.1:8888".to_string(),true, true, 3, 16, 1, 7);
        let mut service = PeerSamplingService::new(config);
        let handle = service.init(Box::new(init_handler));
        handle.join().unwrap();
    }
}
