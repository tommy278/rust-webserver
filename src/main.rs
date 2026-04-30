mod template;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, params, params_from_iter};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex, mpsc::channel};
use std::thread;

enum Method {
    POST,
    GET,
    PUT,
    DELETE,
}

enum Type {
    INTEGER,
    TEXT,
    REAL,
}

impl Type {
    fn to_sql(&self, key: &str) -> String {
        match self {
            Type::INTEGER => format!("{} INTEGER NOT NULL,", key),
            Type::REAL => format!("{} REAL NOT NULL,", key),
            Type::TEXT => format!("{} TEXT NOT NULL,", key),
        }
    }
}

impl FromStr for Type {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.parse::<i64>().is_ok() {
            Ok(Self::INTEGER)
        } else if s.parse::<f64>().is_ok() {
            Ok(Self::REAL)
        } else {
            Ok(Self::TEXT)
        }
    }
}

impl Method {
    fn new(string: &str) -> Self {
        match string {
            "POST" => Method::POST,
            "GET" => Method::GET,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            // TODO: Add better error handling
            _ => panic!("Method not found"),
        }
    }
}

struct HeaderDetails<'a> {
    route: &'a str,
    method: Method,
}

impl HeaderDetails<'_> {
    fn get_header_details(request_header: &str) -> HeaderDetails<'_> {
        let split_header: Vec<&str> = request_header.split_whitespace().collect();

        let mut v_iter = split_header.into_iter();

        let method = v_iter.next().unwrap();
        let route = v_iter.next().unwrap();

        let method = Method::new(method);

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

enum Encoding {
    URL,
    JSON,
}

impl From<&str> for Encoding {
    fn from(value: &str) -> Self {
        match value {
            "application/x-www-form-urlencoded" => Self::URL,
            "application/json" => Self::JSON,
            _ => unreachable!(),
        }
    }
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
    let mut reader = BufReader::new(&stream);

    let lines = (&mut reader).lines();
    let headers: Vec<String> = lines
        .map(|l| l.unwrap())
        .take_while(|l| !l.is_empty())
        .collect();

    let request_header = headers.iter().nth(0).unwrap();

    let HeaderDetails { route, method } = HeaderDetails::get_header_details(request_header);

    let doc_type = DocType::parse_ext(&route);

    match method {
        Method::GET => handle_get_request(&mut stream, route, doc_type, connection),
        Method::POST => {
            let content_length = headers
                .iter()
                .find(|c| c.to_lowercase().starts_with("content-length:"))
                .and_then(|c| c.split_once(':'))
                .map(|(_, value)| value.trim().parse::<usize>().unwrap_or_default())
                .unwrap();

            let content_type = headers
                .iter()
                .find(|c| c.to_lowercase().starts_with("content-type:"))
                .and_then(|c| c.split_once(':'))
                .map(|(_, value)| value.trim())
                .unwrap();

            let content_type = Encoding::from(content_type);

            let mut body_buffer = vec![0; content_length];
            reader.read_exact(&mut body_buffer).unwrap();

            let body: String = String::from_utf8(body_buffer).unwrap();

            handle_post_request(&body, &mut stream, route, connection, content_type)
        }
        Method::PUT => handle_put_request(),
        Method::DELETE => handle_delete_request(&mut stream, route, connection),
    }
}

// Handle POST CREATE ENTRY OR CREATE TABLE

fn handle_put_request() {
    todo!()
}

fn handle_delete_request(stream: &mut TcpStream, route: &str, connection: &Connection) {
    let route = &route[1..];

    let slice_idx = route.find("/").unwrap() as usize + 1;
    let schema = &route[slice_idx..];

    if route.ends_with("delete") {
        let (schema, _) = schema.split_once('/').unwrap();
        let sql = format!("DROP TABLE IF EXISTS {}", schema);
        connection.execute(&sql, ()).unwrap();
    } else {
        let (schema, id) = schema.split_once('/').unwrap();
        let sql = format!("DELETE FROM {} where id = ?1", schema);
        connection.execute(&sql, params![id]).unwrap();
    }

    let response = Response {
        status: 200,
        message: String::from("Succesfully created"),
    };

    let res_json = serde_json::to_string(&response).unwrap();

    let status_header = "HTTP/1.1 200 OK";
    let content_type = "application/json";
    let content_length = res_json.len();

    let response = format!(
        "{status_header}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\n\r\n{res_json}"
    );

    stream.write_all(response.as_bytes()).unwrap();
}

#[derive(Serialize, Deserialize)]
struct Response {
    status: u8,
    message: String,
}

fn handle_post_request(
    body: &str,
    stream: &mut TcpStream,
    route: &str,
    connection: &Connection,
    content_type: Encoding,
) {
    // Remove the beginning char which is / to make the slice idx find accurate
    let route = &route[1..];

    let slice_idx = route.find("/").unwrap() as usize + 1;
    let schema = &route[slice_idx..];

    let mut keys: Vec<String> = Vec::with_capacity(20);
    let mut values: Vec<String> = Vec::with_capacity(20);

    match content_type {
        Encoding::URL => {
            let pairs: Vec<&str> = body.split('&').collect();
            for p in pairs {
                let pair = p.split_once('=').unwrap();
                keys.push(String::from(pair.0));
                values.push(String::from(pair.1));
            }
        }
        Encoding::JSON => {
            let body_json: serde_json::Value = serde_json::from_str(&body).unwrap();
            let obj = body_json.as_object().unwrap();
            for (key, value) in obj {
                keys.push(key.to_string());
                values.push(value.to_string())
            }
        }
    }

    assert_eq!(
        keys.len(),
        values.len(),
        "Something went wrong with parsing"
    );
    // Something like game_id, game_title, ...

    if schema.ends_with("create") {
        handle_create(schema, &keys, &values, connection);

        let (schema, _) = schema.split_once("/").unwrap();
        handle_insert(schema, &keys, &values, connection);
    } else {
        handle_insert(schema, &keys, &values, connection);
    };

    let response = Response {
        status: 200,
        message: String::from("Succesfully created"),
    };

    let res_json = serde_json::to_string(&response).unwrap();

    let status_header = "HTTP/1.1 200 OK";
    let content_type = "application/json";
    let content_length = res_json.len();

    let response = format!(
        "{status_header}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\n\r\n{res_json}"
    );

    stream.write_all(response.as_bytes()).unwrap();
}

fn handle_insert(schema: &str, keys: &Vec<String>, values: &Vec<String>, connection: &Connection) {
    let value_query = keys.join(",");
    let place_holder: Vec<String> = (1..=keys.len()).map(|i| format!("?{}", i)).collect();

    // let values: Vec<&dyn ToSql> = values.iter().map(|v| v as &dyn ToSql).collect();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        schema,
        value_query,
        place_holder.join(",")
    );

    connection.execute(&sql, params_from_iter(values)).unwrap();
}

fn handle_create(schema: &str, keys: &Vec<String>, values: &Vec<String>, connection: &Connection) {
    let (schema, _) = schema.split_once("/").unwrap();

    let mut sql = format!(
        "CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY,",
        schema
    );

    for i in 0..keys.len() {
        let db_type = Type::from_str(&values[i]).unwrap();
        sql.push_str(&format!(" {}", db_type.to_sql(&keys[i])));
    }

    // Remove trailing comma and add a parenthese
    sql.pop();
    sql.push(')');

    connection.execute(&sql, ()).unwrap();
}
// Handle PUT Simply update entry

// DELETE delete entry or table

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

    //  structure like api/* eg api/people or api/games

    if absolute_route.starts_with("api") {
        // Drop the first 4 chars ('api/')
        let schema_name = &absolute_route[4..];

        let query = format!("PRAGMA table_info({})", schema_name);
        let mut stmt = connection.prepare(&query).unwrap();

        let columns: Vec<String> = stmt
            .query_map([], |row| row.get(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        let query = format!("SELECT * FROM {}", schema_name);
        let mut stmt = connection.prepare(&query).unwrap();

        let rows = stmt
            .query_map([], |row| {
                let mut obj = serde_json::Map::new();
                for (i, col) in columns.iter().enumerate() {
                    let value = match row.get_ref(i).unwrap() {
                        ValueRef::Null => serde_json::Value::Null,
                        ValueRef::Integer(i) => serde_json::Value::from(i),
                        ValueRef::Real(f) => serde_json::Value::from(f),
                        ValueRef::Text(s) => {
                            serde_json::Value::from(std::str::from_utf8(s).unwrap_or_default())
                        }
                        ValueRef::Blob(b) => serde_json::Value::from(b),
                    };
                    obj.insert(col.clone(), value);
                }
                Ok(serde_json::Value::Object(obj))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

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
