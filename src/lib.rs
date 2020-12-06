mod monitor;
mod message;
mod network;
mod peer;

pub use crate::peer::Peer;
pub use crate::peer::Config;
pub use crate::peer::PeerSamplingService;

#[cfg(test)]
mod tests {
    use std::thread::JoinHandle;
    use crate::{Peer, Config, PeerSamplingService};
    use std::collections::HashMap;
    use std::error::Error;
    use crate::monitor::MonitoringConfig;

    #[test]
    fn initial_peer() {
        configure_logging("tmp/test.log".to_string(), "INFO".to_string()).unwrap();

        let mut handles = Vec::new();

        for (config, handler) in get_nodes() {
            let handle = start_node(config, handler);
            handles.push(handle);
        };

        let first = handles.remove(0);
        first.join().unwrap();
    }

    fn start_node(config: Config, init_handler: Box<dyn FnOnce() -> Option<Peer>>) -> JoinHandle<()> {
        let mut service = PeerSamplingService::new(config);
        service.init(init_handler)
    }

    fn get_nodes() -> Vec<(Config, Box<dyn FnOnce() -> Option<Peer>>)> {
        let push = true;
        let pull = true;
        let t = 10;
        let d = 10;
        let c = 8;
        let h = 1;
        let s = 3;

        let monitor = Some(MonitoringConfig::new(true, "127.0.0.1:8080/peers"));

        let mut result: Vec<(Config, Box<dyn FnOnce() -> Option<Peer>>)>  = vec![];
        let mut port = 9000;
        let init_port = 9000;
        result.push(
            (Config::new(format!("127.0.0.1:{}", port), push, pull, t, d, c, h, s, monitor.clone()),
             Box::new(|| { None }))
        );
        port += 1;

        for icon in 1..80 {
            let address = format!("127.0.0.1:{}", port);
            port += 1;
            result.push((Config::new(address, push, pull, t, d, c, h, s, monitor.clone()),
                         Box::new(move|| { Some(Peer::new(format!("127.0.0.1:{}", init_port))) })));
        }
        result
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
