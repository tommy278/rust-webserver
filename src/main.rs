use std::fs::File;
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc::channel};
use std::thread;

struct HeaderDetails<'a> {
    route: &'a str,
    method: &'a str,
}

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
    let reader = BufReader::new(&stream);

    let headers: Vec<String> = reader
        .lines()
        .map(|l| l.unwrap())
        .take_while(|l| !l.is_empty())
        .collect();

    let request_header = headers.iter().nth(0).unwrap();

    let doc_type = headers
        .iter()
        .find(|l| l.starts_with("Accept: "))
        .map(|l| &l[13..17]);

    let HeaderDetails { route, method } = get_header_details(request_header);

    let doc_type = parse_doctype(doc_type.unwrap());

    if method == "GET" {
        handle_get_request(&mut stream, route, doc_type);
    }
}

fn parse_doctype(doc_type: &str) -> &'static str {
    if doc_type.starts_with("css") {
        return "css";
    } else if doc_type.starts_with("js") {
        return "js";
    } else if doc_type.starts_with("html") {
        return "html";
    }
    // Return to default HTML for now
    else {
        return "html";
    }
}

fn get_header_details(request_header: &str) -> HeaderDetails<'_> {
    let split_header: Vec<&str> = request_header.split_whitespace().collect();

    let mut v_iter = split_header.into_iter();

    let method = v_iter.next().unwrap();
    let route = v_iter.next().unwrap();

    HeaderDetails { route, method }
}

fn handle_get_request(stream: &mut TcpStream, route: &str, doc_type: &str) {
    let doc_type = parse_doctype(doc_type);

    let absolute_route = match route {
        "/" => format!("static/index.html"),
        _ => {
            // Slicing the route for cleaner formatting
            let route = &route[1..];
            format!("static/{route}.{doc_type}")
        }
    };

    println!("{}", absolute_route);

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
