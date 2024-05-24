use gbps::{terminal_logger, Config, Peer, PeerSamplingService};

// logger for integration tests
struct IntegrationTestLogger;

static LOGGER: IntegrationTestLogger = IntegrationTestLogger;

#[test]
fn sample_code() {

    let logger = terminal_logger();

    let logger_clone = logger.clone();
    let first_handle = std::thread::spawn(|| {
        // configuration
        let config = Config::new("127.0.0.1:9000".parse().unwrap(), true, true, 6, 5, 20, 2, 8, None);

        // closure that returns no contact peer
        let no_initial_peer = Box::new(move|| { None });

        // create and initiate the peer sampling service
        let mut sampling_service = PeerSamplingService::new(config, logger_clone);
        sampling_service.init(no_initial_peer);
        std::thread::sleep(std::time::Duration::from_secs(20));

        // terminate peer sampling
        sampling_service.shutdown().unwrap();
    });

    let logger_clone = logger.clone();
    let second_handle = std::thread::spawn(|| {
        // configuration
        let config = Config::new("127.0.0.1:9001".parse().unwrap(), true, true, 6, 5, 20, 2, 8, None);

        // closure for retrieving the address of the initial contact peer
        let initial_peer = Box::new(move|| { Some(vec![Peer::new("127.0.0.1:9000".to_owned())]) });

        // create and initiate the peer sampling service
        let mut sampling_service = PeerSamplingService::new(config, logger_clone);
        sampling_service.init(initial_peer);
        std::thread::sleep(std::time::Duration::from_secs(20));

        // terminate peer sampling
        sampling_service.shutdown().unwrap();
    });

    first_handle.join().unwrap();
    second_handle.join().unwrap();
}