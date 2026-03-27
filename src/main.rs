use std::fmt::Debug;
use std::fs::{self, File};
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let reader = BufReader::new(&stream);
    let request_header = reader.lines().next().unwrap().unwrap();

    if request_header == "GET / HTTP/1.1" {
        let mut buf = String::new();
        let mut file = File::open("src/index.html").unwrap();
        file.read_to_string(&mut buf).unwrap();

        let length = buf.len();

        let status_header = "HTTP/1.1 200 OK";

        let response = format!(
            "{status_header}\r\nContent-Type: text/html\r\nContent-Length: {length}\r\n\r\n{buf}"
        );

        stream.write_all(response.as_bytes()).unwrap();
    }
}
