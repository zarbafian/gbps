#[cfg(test)]
mod tests {
    use log::{Metadata, Level, Record, LevelFilter};

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
    fn peer_sampling_smoke_test() {
        use gbps::{Config, MonitoringConfig, PeerSamplingService, Peer};

        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(LevelFilter::Debug)).unwrap();

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
        let mut instances = vec![];

        // create first peer with no contact peer
        let init_address = "127.0.0.1:9000";
        // configuration
        let first_config = Config::new(init_address.parse().unwrap(), push, pull, t, d, c, h, s, Some(monitoring_config.clone()));
        // no contact peer for first node
        let no_peer_handler = Box::new(move|| { None });

        // create and initiate the peer sampling service
        let mut service = PeerSamplingService::new(first_config);
        service.init(no_peer_handler);
        instances.push(service);

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
            let mut ipv4_service = PeerSamplingService::new(config);
            ipv4_service.init(init_handler);
            instances.push(ipv4_service);

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
            let mut ipv6_service = PeerSamplingService::new(config);
            ipv6_service.init(init_handler);
            instances.push(ipv6_service);

            port += 1;
        }

        std::thread::sleep(std::time::Duration::from_secs(11));

        assert!(&instances[0].get_peer().is_some());

        for mut instance in instances {
            instance.shutdown().unwrap();
        }
    }

    #[test]
    fn does_shutdown() {
        use gbps::{Config, PeerSamplingService};

        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(LevelFilter::Debug)).unwrap();

        // configuration
        let config = Config::new("127.0.0.1:9000".parse().unwrap(), true, true, 1, 0, 20, 2, 8, None);
        // closure for retrieving the address of the first contact peer
        let init_handler = Box::new(move|| { None });

        // create and initiate the peer sampling service
        let mut service = PeerSamplingService::new(config);
        service.init(init_handler);

        std::thread::sleep(std::time::Duration::from_secs(3));
        service.shutdown().unwrap();
    }
}