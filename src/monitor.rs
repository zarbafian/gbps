#[derive(Clone)]
pub struct MonitoringConfig {
    enabled: bool,
    host: String,
    context: String,
}

impl MonitoringConfig {
    pub fn new(enabled: bool, url: &str) -> MonitoringConfig {
        // remove leading protocol
        let protocol_removed = match url.find("://") {
            Some(index) => &url[index+3..],
            None => url
        };
        // separate host and context
        let (host, context) = match protocol_removed.find("/") {
            Some(index) => (&url[..index], &url[index..]),
            None => (url, "/")
        };
        MonitoringConfig {
            enabled,
            host: host.to_owned(),
            context: context.to_owned()
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        MonitoringConfig {
            enabled: false,
            host: "".to_string(),
            context: "".to_string()
        }
    }
}

pub fn send_data(config: MonitoringConfig, pid: &str, peers: Vec<String>) {

    let pid = pid.to_owned();
    std::thread::spawn(move|| {
        let peers_str = peers.iter()
            .map(|peer| format!("\"{}\"", peer))
            .collect::<Vec<String>>().join(",");
        let json = format!(
            "{{\
                \"id\":\"{}\",\
                \"peers\":[{}],\
                \"messages\":[{}]\
            }}", pid, peers_str, "");
        //println!("send_data:\n{}", json);
        if let Ok(()) = post(json) {
            log::debug!("Peer {}: monitoring data sent", pid);
        }
        else {
            log::warn!("Peer {}: could not send monitoring data", pid);
        }
    });
}

fn post(config: MonitoringConfig, json: String) -> std::io::Result<()> {
    use std::io::Read;
    use std::io::Write;

    let bytes = json.as_bytes();

    let mut stream = std::net::TcpStream::connect(config.host)?;

    let mut request_data = String::new();
    request_data.push_str(&format!("POST {} HTTP/1.1", config.context));
    request_data.push_str("\r\n");
    request_data.push_str(&format!("Host: {}", config.host));
    request_data.push_str("\r\n");
    request_data.push_str("Accept: */*");
    request_data.push_str("\r\n");
    request_data.push_str("Content-Type: application/json; charset=UTF-8");
    request_data.push_str("\r\n");
    request_data.push_str(&format!("Content-Length: {}", bytes.len()));
    request_data.push_str("\r\n");
    request_data.push_str("Connection: close");
    request_data.push_str("\r\n");
    request_data.push_str("\r\n");
    request_data.push_str(&json);

    //println!("request_data = {:?}", request_data);

    let request = stream.write_all(request_data.as_bytes())?;
    //println!("request = {:?}", request);

    let mut buf = String::new();
    let result = stream.read_to_string(&mut buf)?;
    //println!("result = {}", result);
    //println!("buf = {}", buf);

    Ok(())
}