use gbps::terminal_logger;
use slog::{o, Logger};

// logger for integration tests
struct IntegrationTestLogger;

static LOGGER: IntegrationTestLogger = IntegrationTestLogger;

#[test]
fn peer_sampling_smoke_test() {
    use gbps::{Config, MonitoringConfig, PeerSamplingService, Peer};

    let logger = terminal_logger();
    
    // algorithm parameters
    let push = true;
    let pull = true;
    let t = 5;
    let d = 5;
    let c = 4;
    let h = 1;
    let s = 2;

    let monitoring_config = MonitoringConfig::new(true, "http://127.0.0.1:8080/peers", logger.clone());

    let peers_per_protocol = 5;
    let mut instances = vec![];

    // create first peer with no contact peer
    let init_address = "127.0.0.1:9000";
    // configuration
    let first_config = Config::new(init_address.parse().unwrap(), push, pull, t, d, c, h, s, Some(monitoring_config.clone()));
    // no contact peer for first node
    let no_peer_handler = Box::new(move|| { None });

    // create and initiate the peer sampling service
    let mut service = PeerSamplingService::new(first_config, logger.clone());
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
        let init_handler = Box::new(move|| { Some(vec![Peer::new(init_address.to_owned())]) });

        // create and initiate the peer sampling service
        let mut ipv4_service = PeerSamplingService::new(config, logger.clone());
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
        let init_handler = Box::new(move|| { Some(vec![Peer::new(init_address.to_owned())]) });

        // create and initiate the peer sampling service
        let mut ipv6_service = PeerSamplingService::new(config, logger.clone());
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

    let logger = terminal_logger();

    // configuration
    let config = Config::new("127.0.0.1:9000".parse().unwrap(), true, true, 1, 0, 20, 2, 8, None);
    // closure for retrieving the address of the first contact peer
    let init_handler = Box::new(move|| { None });

    // create and initiate the peer sampling service
    let mut service = PeerSamplingService::new(config, logger.clone());
    service.init(init_handler);

    std::thread::sleep(std::time::Duration::from_secs(3));
    service.shutdown().unwrap();
}