pub const MONITORING_ENABLED: bool = true;
pub const MONITORING_HOST: &str = "127.0.0.1:8080";
pub const MONITORING_CONTEXT: &str = "/peers";

pub fn send_data(pid: &str, peers: Vec<String>) {

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
        post(json);
        log::debug!("Peer {}: monitoring data sent", pid);
    });
}

fn post(json: String) -> std::io::Result<()> {
    use std::io::Read;
    use std::io::Write;

    let bytes = json.as_bytes();

    let mut stream = std::net::TcpStream::connect(MONITORING_HOST)?;

    let mut request_data = String::new();
    request_data.push_str(&format!("POST {} HTTP/1.1", MONITORING_CONTEXT));
    request_data.push_str("\r\n");
    request_data.push_str(&format!("Host: {}", MONITORING_HOST));
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