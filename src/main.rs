use std::fs::File;
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc::channel};
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    let (tx, rx) = channel::<TcpStream>();
    let reciever = Arc::new(Mutex::new(rx));

    for _ in 0..4 {
        let reciever = reciever.clone();
        thread::spawn(move || {
            loop {
                let stream = reciever.lock().unwrap().recv().unwrap();
                handle_connection(stream);
            }
        });
    }

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        tx.send(stream).unwrap();
    }
}

fn handle_connection(mut stream: TcpStream) {
    std::thread::sleep(std::time::Duration::from_secs(3));
    let reader = BufReader::new(&stream);
    let request_header = reader.lines().next().unwrap().unwrap();

    let split_header: Vec<&str> = request_header.split_whitespace().collect();

    let mut v_iter = split_header.into_iter();

    let method = v_iter.next().unwrap();
    let route = v_iter.next().unwrap();

    if method == "GET" {
        let absolute_route = match route {
            "/" => format!("static/index.html"),
            _ => format!("static{route}.html"),
        };

        let mut buf = String::new();
        let status_header: &str;
        let length: usize;

        if let Ok(mut file) = File::open(absolute_route) {
            file.read_to_string(&mut buf).unwrap();
            length = buf.len();
            status_header = "HTTP/1.1 200 OK";
        } else {
            let mut file = File::open("static/not-found.html").unwrap();
            file.read_to_string(&mut buf).unwrap();
            length = buf.len();
            status_header = "HTTP/1.1 404 Not Found";
        }

        let response = format!(
            "{status_header}\r\nContent-Type: text/html\r\nContent-Length: {length}\r\n\r\n{buf}"
        );

        stream.write_all(response.as_bytes()).unwrap();
    }
}
