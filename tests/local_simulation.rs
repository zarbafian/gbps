#[cfg(test)]
mod tests {
    use log::{Metadata, Level, Record, LevelFilter};
    use gbps::MonitoringConfig;

    // logger for integration tests
    struct IntegrationTestLogger;

    // basic implementation that prints to stdout
    impl log::Log for IntegrationTestLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Debug
        }

        fn log(&self, record: &Record) {
            if self.enabled(record.metadata()) {
                println!("{:?} - {}", record.level(), record.args());
            }
        }

        fn flush(&self) {}
    }

    static LOGGER: IntegrationTestLogger = IntegrationTestLogger;

    #[test]
    fn start_nodes() {
        use gbps::{Config, PeerSamplingService, Peer};


        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(log::LevelFilter::Debug)).unwrap();

        // algorithm parameters
        let push = true;
        let pull = true;
        let t = 5;
        let d = 5;
        let c = 4;
        let h = 1;
        let s = 2;

        let monitoring_config = MonitoringConfig::new(true, "http://127.0.0.1:8080/peers");

        let peers_per_protocol = 5;

        // create first peer with no contact peer
        let init_address = "127.0.0.1:9000";
        // configuration
        let first_config = Config::new(init_address.parse().unwrap(), push, pull, t, d, c, h, s, Some(monitoring_config.clone()));
        // no contact peer for first node
        let no_peer_handler = Box::new(move|| { None });

        // create and initiate the peer sampling service
        let mut service = PeerSamplingService::new(first_config);
        let mut _join_handles = service.init(no_peer_handler);

        // create peers using IPv4 addresses
        let mut port = 9001;
        for _ in 1..peers_per_protocol {
            // peer socket address
            let address = format!("127.0.0.1:{}", port);
            // configuration
            let config = Config::new(address.parse().unwrap(), push, pull, t, d, c, h, s, Some(monitoring_config.clone()));
            // closure for retrieving the address of the first contact peer
            let init_handler = Box::new(move|| { Some(Peer::new(init_address.to_owned())) });

            // create and initiate the peer sampling service
            let _handles = PeerSamplingService::new(config).init(init_handler);
            port += 1;
        }

        // create peers using IPv6 addresses
        for _ in 1..peers_per_protocol {
            // peer socket address
            let address = format!("[::1]:{}", port);
            // configuration
            let config = Config::new(address.parse().unwrap(), push, pull, t, d, c, h, s, Some(monitoring_config.clone()));
            // closure for retrieving the address of the first contact peer
            let init_handler = Box::new(move|| { Some(Peer::new(init_address.to_owned())) });

            // create and initiate the peer sampling service
            let _handles = PeerSamplingService::new(config).init(init_handler);
            port += 1;
        }

        std::thread::sleep(std::time::Duration::from_secs(50));

        assert!(service.get_peer().is_some());
    }
}