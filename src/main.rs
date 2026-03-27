use std::fmt::Debug;
use std::fs::{self, File};
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::ops::Index;

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
