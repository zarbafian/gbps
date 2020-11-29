mod network;
mod conf;
mod peer;

#[cfg(test)]
mod tests {
    use crate::peer::{Config, PeerSamplingService, Peer};
    use crate::conf;

    #[test]
    fn round() {
        let v = 23 / 2 - 1;
        println!("v: {}", v);
    }

    #[test]
    fn shuffle() {
        let mut a = vec![Peer::new("tutu".to_string()), Peer::new("toot".to_string()), Peer::new("titi".to_string()), Peer::new("tata".to_string())];
        // 2, 1, 3, 0
        a[0].increment_age();
        a[0].increment_age();
        a[1].increment_age();
        a[2].increment_age();
        a[2].increment_age();
        a[2].increment_age();

        let mut sorted_by_age = a.clone();
        sorted_by_age.sort_by_key(|p| p.get_age());
        sorted_by_age.reverse();
        sorted_by_age.truncate(2);

        println!("initial: {:?}", a);
        println!("  moved: {:?}", sorted_by_age);

        let mut view_start = vec![];
        let mut view_end = vec![];
        for peer in a {
            if sorted_by_age.contains(&peer) {
                view_end.push(peer.clone());
            }
            else {
                view_start.push(peer.clone());
            }
        }
        view_start.append(&mut view_end);
        println!("    new: {:?}", view_start);
    }

    #[test]
    fn it_works() {

        conf::configure_logging("tmp/test.log".to_string(), "DEBUG".to_string()).unwrap();

        let init_handler = || {
            Peer::new("127.0.0.1:11111".to_string())
        };

        let config = Config::new("127.0.0.1:8888".to_string(),true, true, 3, 16, 1, 7);
        let mut service = PeerSamplingService::new(config);
        service.init(Box::new(init_handler));
    }
}
