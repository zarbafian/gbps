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
    use std::thread::JoinHandle;
    use crate::{Peer, Config, PeerSamplingService};
    use std::collections::{HashMap, VecDeque};
    use std::error::Error;
    use crate::monitor::MonitoringConfig;

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

        let monitor = Some(MonitoringConfig::new(true, "127.0.0.1:8080/peers"));
        configure_logging("tmp/test.log".to_owned(), "INFO".to_owned());

        let mut result: Vec<(Config, Box<dyn FnOnce() -> Option<Peer>>)>  = vec![];

        // create first peer with no contact peer
        let init_address = "127.0.0.1:9000";
        // configuration
        let first_config = Config::new(init_address.parse().unwrap(), push, pull, t, d, c, h, s, monitor.clone());
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
            let config = Config::new(address.parse().unwrap(), push, pull, t, d, c, h, s, monitor.clone());
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
            let config = Config::new(address.parse().unwrap(), push, pull, t, d, c, h, s, monitor.clone());
            // closure for retrieving the address of the first contact peer
            let init_handler = Box::new(move|| { Some(Peer::new(init_address.to_owned())) });

            // create and initiate the peer sampling service
            let _handles = PeerSamplingService::new(config).init(init_handler);
            port += 1;
        }

        join_handles.remove(0).join();
    }

    fn configure_logging(file: String, level: String) -> Result<(), Box<dyn Error>>{

        use log::{LevelFilter};
        use log4rs::append::file::FileAppender;
        use log4rs::encode::pattern::PatternEncoder;
        use log4rs::config::{Appender, Config, Root};
        use log4rs::append::console::ConsoleAppender;

        let mut log_levels: HashMap<String, LevelFilter> = HashMap::new();
        log_levels.insert(String::from("OFF"), LevelFilter::Off);
        log_levels.insert(String::from("ERROR"), LevelFilter::Error);
        log_levels.insert(String::from("WARN"), LevelFilter::Warn);
        log_levels.insert(String::from("INFO"), LevelFilter::Info);
        log_levels.insert(String::from("DEBUG"), LevelFilter::Debug);
        log_levels.insert(String::from("TRACE"), LevelFilter::Trace);

        let mut level_filter = &LevelFilter::Info;

        if let Some(filter) = log_levels.get(&level.to_uppercase()) {
            level_filter = filter;
        }
        else {
            Err(format!("Invalid logging level: {}", level))?
        }

        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{d} [{l}] {T} - {m}{n}")))
            .build(file)?;

        let console = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new("[{l}] {T} - {m}{n}")))
            .build();

        let config = Config::builder()
            .appender(Appender::builder().build("console", Box::new(console)))
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(Root::builder()
                .appender("console")
                .appender("logfile")
                .build(*level_filter))?;

        log4rs::init_config(config)?;

        Ok(())
    }
}
