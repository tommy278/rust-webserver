use std::fs::{self, File};
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc::channel};
use std::thread;

struct HeaderDetails<'a> {
    route: &'a str,
    method: &'a str,
}

impl HeaderDetails<'_> {
    fn get_header_details(request_header: &str) -> HeaderDetails<'_> {
        let split_header: Vec<&str> = request_header.split_whitespace().collect();

        let mut v_iter = split_header.into_iter();

        let method = v_iter.next().unwrap();
        let route = v_iter.next().unwrap();

        HeaderDetails { route, method }
    }
}

#[derive(Debug)]
enum DocType {
    CSS,
    JS,
    HTML,
    OTHER,
}

impl DocType {
    fn parse_doctype(doc_type: &str) -> DocType {
        if doc_type.starts_with("css") {
            return DocType::CSS;
        } else if doc_type.starts_with("js") {
            return DocType::JS;
        } else if doc_type.starts_with("html") {
            return DocType::HTML;
        }
        // Return to default HTML for now
        else {
            return DocType::OTHER;
        }
    }
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

    let doc_type = parse_ext(&headers);
    println!("{:?}", doc_type);

    let HeaderDetails { route, method } = HeaderDetails::get_header_details(request_header);

    println!("{:?}", doc_type);

    if method == "GET" {
        handle_get_request(&mut stream, route, doc_type);
    }
}

fn parse_ext(headers: &Vec<String>) -> DocType {
    let doc_type = headers
        .iter()
        .find(|l| l.starts_with("Accept: "))
        .map(|l| &l[8..])
        .unwrap();

    println!("Here:{}", doc_type);
    if doc_type.starts_with("*/*") {
        return DocType::OTHER;
    } else {
        let doc_type = &doc_type[5..9];

        println!("bottom {}", doc_type);

        if doc_type.starts_with("html") {
            return DocType::HTML;
        } else if doc_type.starts_with("css") {
            return DocType::CSS;
        } else if doc_type.starts_with("js") {
            return DocType::JS;
        } else {
            return DocType::OTHER;
        }
    }
}

fn handle_get_request(stream: &mut TcpStream, route: &str, doc_type: DocType) {
    let absolute_route = match route {
        "/" => format!("static/index.html"),
        _ => {
            // Slicing the route for cleaner formatting
            let route = &route[1..];
            println!("This is the route {}", route);
            match doc_type {
                DocType::HTML => format!("static/{route}.html"),
                DocType::JS | DocType::CSS | DocType::OTHER => route.to_string(),
            }
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
