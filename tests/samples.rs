use gbps::{Config, PeerSamplingService, Peer};

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
fn sample_code() {

    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Debug)).unwrap();

    let first_handle = std::thread::spawn(|| {
        // configuration
        let config = Config::new("127.0.0.1:9000".parse().unwrap(), true, true, 6, 5, 20, 2, 8, None);

        // closure that returns no contact peer
        let no_initial_peer = Box::new(move|| { None });

        // create and initiate the peer sampling service
        let mut sampling_service = PeerSamplingService::new(config);
        sampling_service.init(no_initial_peer);
        std::thread::sleep(std::time::Duration::from_secs(20));

        // terminate peer sampling
        sampling_service.shutdown().unwrap();
    });

    let second_handle = std::thread::spawn(|| {
        // configuration
        let config = Config::new("127.0.0.1:9001".parse().unwrap(), true, true, 6, 5, 20, 2, 8, None);

        // closure for retrieving the address of the initial contact peer
        let initial_peer = Box::new(move|| { Some(vec![Peer::new("127.0.0.1:9000".to_owned())]) });

        // create and initiate the peer sampling service
        let mut sampling_service = PeerSamplingService::new(config);
        sampling_service.init(initial_peer);
        std::thread::sleep(std::time::Duration::from_secs(20));

        // terminate peer sampling
        sampling_service.shutdown().unwrap();
    });

    first_handle.join().unwrap();
    second_handle.join().unwrap();
}