mod template;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
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
    API,
    OTHER,
}

impl DocType {
    fn parse_ext(route: &str) -> DocType {
        if route.ends_with(".css") {
            return DocType::CSS;
        } else if route.ends_with(".js") {
            return DocType::JS;
        } else if route.starts_with("/api") {
            return DocType::API;
        } else if (route.starts_with("/") && !route.contains(".")) || route.ends_with("/") {
            return DocType::HTML;
        } else {
            return DocType::OTHER;
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Person {
    id: i32,
    name: String,
}

struct ThreadData {
    stream: TcpStream,
    connection: Connection,
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    let (tx, rx) = channel::<ThreadData>();
    let reciever = Arc::new(Mutex::new(rx));

    for _ in 0..4 {
        let reciever = reciever.clone();
        thread::spawn(move || {
            loop {
                let data = reciever.lock().unwrap().recv().unwrap();
                handle_connection(data.stream, &data.connection);
            }
        });
    }

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let path = Path::new("api/store.db3");
        let connection = Connection::open(path).unwrap();
        tx.send(ThreadData { stream, connection }).unwrap();
    }
}

fn handle_connection(mut stream: TcpStream, connection: &Connection) {
    let reader = BufReader::new(&stream);

    let headers: Vec<String> = reader
        .lines()
        .map(|l| l.unwrap())
        .take_while(|l| !l.is_empty())
        .collect();

    let request_header = headers.iter().nth(0).unwrap();

    let HeaderDetails { route, method } = HeaderDetails::get_header_details(request_header);

    let doc_type = DocType::parse_ext(&route);

    if method == "GET" {
        handle_get_request(&mut stream, route, doc_type, connection);
    }
}

fn handle_get_request(
    stream: &mut TcpStream,
    route: &str,
    doc_type: DocType,
    connection: &Connection,
) {
    let absolute_route = match route {
        "/" => "static/index.html".to_string(),
        _ => {
            // Slicing the route for cleaner formatting
            let route = &route[1..];
            match doc_type {
                DocType::HTML => format!("static/{route}.html"),
                DocType::API | DocType::JS | DocType::CSS | DocType::OTHER => route.to_string(),
            }
        }
    };

    let is_safe = absolute_route.starts_with("static/")
        || absolute_route.starts_with("styles/")
        || absolute_route.starts_with("scripts/")
        || absolute_route.starts_with("api/");

    if !is_safe {
        let err = "HTTP/1.1 403 Forbidden\r\n\r\n";
        stream.write_all(err.as_bytes()).unwrap();
        return;
    }

    if absolute_route.starts_with("api") {
        let mut stmt = connection.prepare("SELECT id, name FROM persons").unwrap();
        let raw_rows = stmt
            .query_map([], |row| {
                Ok(Person {
                    id: row.get_unwrap(0),
                    name: row.get_unwrap(1),
                })
            })
            .unwrap();

        let mut rows: Vec<serde_json::Value> = Vec::with_capacity(20);

        for person in raw_rows {
            match person {
                Ok(person) => rows.push(serde_json::json!(
                    {
                        "id": person.id,
                        "name": person.name
                    }
                )),
                Err(e) => eprintln!("{:?}", e),
            }
        }

        let rows_json = serde_json::json!(rows).to_string();

        let status_header = "HTTP/1.1 200 OK";
        let content_type = "application/json";
        let content_length = rows_json.len();

        let response = format!(
            "{status_header}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\n\r\n{rows_json}"
        );

        stream.write_all(response.as_bytes()).unwrap();
        return;
    }

    let mut buf = String::new();
    let status_header: &str;
    let length: usize;

    let content_type = match doc_type {
        DocType::CSS => "text/css",
        DocType::HTML => "text/html",
        DocType::JS => "text/javascript",
        DocType::OTHER => "application/octet-stream",
        DocType::API => unreachable!(),
    };

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
        "{status_header}\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\n\r\n{buf}"
    );

    stream.write_all(response.as_bytes()).unwrap();
}
