use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::net::TcpStream;
use std::str::FromStr;

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub status: Status,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub enum Status {
    Ok = 200,
    Created = 201,
    BadRequest = 400,
    Forbidden = 403,
    NotFound = 404,
    InternalServerError = 500,
}

impl Status {
    pub fn status_header(&self) -> &str {
        match self {
            Status::Ok => "HTTP/1.1 200 OK",
            Status::Created => "HTTP/1.1 201 Created",
            Status::BadRequest => "HTTP/1.1 400 BadRequest",
            Status::Forbidden => "HTTP/1.1 403 Forbidden",
            Status::NotFound => "HTTP/1.1 404 NotFound",
            Status::InternalServerError => "HTTP/1.1 500 InternalServerError",
        }
    }
}

impl Response {
    pub fn send_response(&self, stream: &mut TcpStream, content: &str, content_type: DocType) {
        let status_header = self.status.status_header();
        let content_length = content.len();
        let content_type = content_type.to_string();

        let response = format!(
            "{status_header}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\n\r\n{content}"
        );

        stream.write_all(response.as_bytes()).unwrap();
    }
}

pub enum Method {
    POST,
    GET,
    PUT,
    DELETE,
}

impl Method {
    pub fn new(string: &str) -> Self {
        match string {
            "POST" => Method::POST,
            "GET" => Method::GET,
            "PUT" | "PATCH" => Method::PUT,
            "DELETE" => Method::DELETE,
            _ => panic!("Method not found"),
        }
    }
}

pub enum Type {
    INTEGER,
    TEXT,
    REAL,
    BLOB,
    NULL,
}

impl Type {
    pub fn to_sql(&self, key: &str) -> String {
        format!("{} {},", key, self.as_str())
    }
    pub fn as_str(&self) -> &str {
        match self {
            Type::INTEGER => "INTEGER",
            Type::TEXT => "TEXT",
            Type::REAL => "REAL",
            Type::BLOB => "BLOB",
            Type::NULL => "NULL",
        }
    }
}

impl FromStr for Type {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Trim for literal quotes
        match s.to_lowercase().trim_matches('"') {
            "integer" => Ok(Self::INTEGER),
            "text" => Ok(Self::TEXT),
            "real" | "float" => Ok(Self::REAL),
            "blob" => Ok(Type::BLOB),
            "null" => Ok(Self::NULL),
            _ => todo!(),
        }
    }
}

pub struct HeaderDetails<'a> {
    pub route: &'a str,
    pub method: Method,
}

impl HeaderDetails<'_> {
    pub fn get_header_details(request_header: &str) -> HeaderDetails<'_> {
        let split_header: Vec<&str> = request_header.split_whitespace().collect();

        let mut v_iter = split_header.into_iter();

        let method = v_iter.next().unwrap();
        let route = v_iter.next().unwrap();

        let method = Method::new(method);

        HeaderDetails { route, method }
    }
}

#[derive(Debug)]
pub enum DocType {
    CSS,
    JS,
    HTML,
    API,
    OTHER,
}

impl DocType {
    pub fn parse_ext(route: &str) -> DocType {
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

    pub fn to_string(&self) -> &str {
        match self {
            DocType::CSS => "text/css",
            DocType::HTML => "text/html",
            DocType::JS => "text/javascript",
            DocType::OTHER => "application/octet-stream",
            DocType::API => "application/json",
        }
    }
}

pub enum Encoding {
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

pub struct ThreadData {
    pub stream: TcpStream,
    pub connection: Connection,
}
