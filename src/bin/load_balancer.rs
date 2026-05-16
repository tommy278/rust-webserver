use http::server;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU16, Ordering},
    mpsc::channel,
};
use std::thread;

const MAX_PORT_NUM: u16 = 5;

fn main() {
    let mut ports = vec![0; MAX_PORT_NUM.into()];

    for i in 0..MAX_PORT_NUM {
        ports[i as usize] = 8081 + i;
    }

    let port_num = Arc::new(AtomicU16::new(8081));

    for port in ports {
        thread::spawn(move || {
            server::run(port);
        });
    }

    let listener = TcpListener::bind("0.0.0.0:8080").expect("Failed to bind load balancer");
    let (tx, rx) = channel::<TcpStream>();
    let reciever = Arc::new(Mutex::new(rx));

    for _ in 0..4 {
        let reciever = reciever.clone();
        let port_num = port_num.clone();
        thread::spawn(move || {
            loop {
                let stream = reciever.lock().unwrap().recv().unwrap();

                // Wrap around on overflow in release mode
                let current = port_num.fetch_add(1, Ordering::Relaxed);
                let port = (current % MAX_PORT_NUM) + 8081;

                forward(stream, &format!("0.0.0.0:{}", port));
            }
        });
    }

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        tx.send(stream).unwrap();
    }
}

fn forward(mut client: TcpStream, backend_addr: &str) {
    let mut backend = TcpStream::connect(backend_addr).unwrap();
    // Read from client, write to backend

    let res = extract_request(&mut client);
    backend.write_all(&res).unwrap();

    // Read response from backend
    let res = extract_request(&mut backend);
    client.write_all(&res).unwrap()
}

fn extract_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut reader = BufReader::new(stream);
    let mut request = Vec::new();

    let mut line = String::new();
    let mut content_length = 0;

    loop {
        line.clear();
        reader.read_line(&mut line).unwrap();

        if line == "\r\n" {
            request.extend_from_slice(b"\r\n");
            break;
        }

        if line.to_lowercase().starts_with("content-length:") {
            content_length = line
                .split_once(":")
                .unwrap()
                .1
                .trim()
                .parse()
                .unwrap_or_default();
        }
        request.extend_from_slice(line.as_bytes());
    }

    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).unwrap();
    request.extend_from_slice(&body);

    request
}
