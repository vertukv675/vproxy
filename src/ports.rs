use std::net::TcpStream;

pub fn port_is_used(port: u16) -> bool {
    match TcpStream::connect(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
