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
        conf::configure_logging("tmp/test.log".to_string(), "INFO".to_string()).unwrap();

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
        let t = 5;
        let c = 6;
        let h = 1;
        let s = 2;

        let mut result: Vec<(Config, Box<dyn FnOnce() -> Option<Peer>>)>  = vec![];
        let mut port = 9000;
        let init_port = 9000;
        result.push(
            (Config::new(format!("127.0.0.1:{}", port), push, pull, t, c, h, s),
             Box::new(|| { None }))
        );
        port += 1;

        for icon in 1..33 {
            let address = format!("127.0.0.1:{}", port);
            port += 1;
            result.push((Config::new(address, push, pull, t, c, h, s),
                         Box::new(move|| { Some(Peer::new(format!("127.0.0.1:{}", init_port))) })));
        }
        result
    }
}
