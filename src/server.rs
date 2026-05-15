use crate::error::server_error::ServerError;
use crate::interface::{
    DocType, Encoding, HeaderDetails, Method, Response, Status, ThreadData, Type,
};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, params, params_from_iter};
use std::fs::File;
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::result::Result;
use std::str::FromStr;
use std::sync::{Arc, Mutex, mpsc::channel};
use std::thread;

macro_rules! ensure {
    ($cond:expr, $msg:expr) => {
        if !$cond {
            return Err(ServerError::ParseError($msg.to_string()));
        }
    };
}

pub fn run(port: u16) {
    let port = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&port).expect(&format!("Failed to bind to port: {}", port));
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

        println!("Request handled by port: {}", port);
    }
}

fn handle_connection(stream: TcpStream, connection: &Connection) {
    if let Err(e) = try_handle_connection(stream, connection) {
        eprintln!("{e}");
    }
}

fn try_handle_connection(
    mut stream: TcpStream,
    connection: &Connection,
) -> Result<(), ServerError> {
    let mut reader = BufReader::new(&stream);

    let lines = (&mut reader).lines();
    let headers: Vec<String> = lines
        .map(|l| l.unwrap())
        .take_while(|l| !l.is_empty())
        .collect();

    let request_header = headers.first().ok_or(ServerError::ParseError(
        "Could not find request header".to_string(),
    ))?;

    let HeaderDetails { route, method } = HeaderDetails::get_header_details(request_header);

    let doc_type = DocType::parse_ext(&route);

    match method {
        Method::GET => handle_get_request(&mut stream, route, doc_type, connection)?,
        Method::POST => {
            let (body, content_type) = parse_body(&headers, &mut reader)?;
            handle_post_request(&body, &mut stream, route, connection, content_type)?
        }
        Method::PUT => {
            let (body, content_type) = parse_body(&headers, &mut reader)?;
            handle_put_request(&body, &mut stream, route, connection, content_type)?
        }
        Method::DELETE => handle_delete_request(&mut stream, route, connection)?,
    };

    Ok(())
}

// Handle POST CREATE ENTRY OR CREATE TABLE

fn handle_put_request(
    body: &str,
    stream: &mut TcpStream,
    route: &str,
    connection: &Connection,
    content_type: Encoding,
) -> Result<(), ServerError> {
    let route = &route[1..];

    let slice_idx = find_by_pat(route, '/')? + 1;
    let schema = &route[slice_idx..];

    let mut keys: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();

    let (schema, id) = split_by_delimeter(schema, '/')?;

    match content_type {
        Encoding::URL => {
            let pairs: Vec<&str> = body.split('&').collect();
            for p in pairs {
                let pair = split_by_delimeter(p, '=')?;
                keys.push(String::from(pair.0));
                values.push(String::from(pair.1));
            }
        }
        Encoding::JSON => {
            let body_json: serde_json::Value = serde_json::from_str(&body)?;
            let obj = body_json.as_object().unwrap();
            for (key, value) in obj {
                keys.push(key.to_string());
                values.push(value.to_string())
            }
        }
    }

    ensure!(
        keys.len() == values.len(),
        "Something went wrong with parsing"
    );
    let mut sql = format!("UPDATE {} SET ", schema);

    for (i, key) in keys.iter().enumerate() {
        let field = format!("{} = {}", key, &format!("?{}", i + 1));
        sql.push_str(&field);
    }

    // The id field will be the last item in the vec
    // It is yet to be inserted, insertion occurs right after
    let id_field = format!(" WHERE id = ?{}", values.len() + 1);
    values.push(id.to_string());

    sql.push_str(&id_field);

    connection.execute(&sql, params_from_iter(values))?;

    let response = Response {
        status: Status::Ok,
        message: String::from("Succesfully updated"),
    };

    let res_json = serde_json::to_string(&response)?;
    response.send_response(stream, &res_json, DocType::API);

    Ok(())
}

fn handle_delete_request(
    stream: &mut TcpStream,
    route: &str,
    connection: &Connection,
) -> Result<(), ServerError> {
    let route = &route[1..];

    let slice_idx = find_by_pat(route, '/')? + 1;
    let schema = &route[slice_idx..];

    if route.ends_with("delete") {
        let (schema, _) = split_by_delimeter(schema, '/')?;
        let sql = format!("DROP TABLE IF EXISTS {}", schema);
        connection.execute(&sql, ())?;
    } else {
        let (schema, id) = split_by_delimeter(schema, '/')?;
        let sql = format!("DELETE FROM {} WHERE id = ?1", schema);
        connection.execute(&sql, params![id])?;
    }

    let response = Response {
        status: Status::Ok,
        message: String::from("Succesfully deleted"),
    };

    let res_json = serde_json::to_string(&response)?;
    response.send_response(stream, &res_json, DocType::API);
    Ok(())
}

fn handle_post_request(
    body: &str,
    stream: &mut TcpStream,
    route: &str,
    connection: &Connection,
    content_type: Encoding,
) -> Result<(), ServerError> {
    // Remove the beginning char which is / to make the slice idx find accurate
    let route = &route[1..];

    let slice_idx = find_by_pat(route, '/')? + 1;
    let schema = &route[slice_idx..];

    let mut keys: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();

    match content_type {
        Encoding::URL => {
            let pairs: Vec<&str> = body.split('&').collect();
            for p in pairs {
                let pair = split_by_delimeter(p, '=')?;
                keys.push(String::from(pair.0));
                values.push(String::from(pair.1));
            }
        }
        Encoding::JSON => {
            let body_json: serde_json::Value = serde_json::from_str(&body)?;
            let obj = body_json.as_object().unwrap();
            for (key, value) in obj {
                keys.push(key.to_string());
                values.push(value.to_string())
            }
        }
    }

    ensure!(
        keys.len() == values.len(),
        "Something went wrong with parsing"
    );
    // Something like game_id, game_title, ...

    if schema.ends_with("create") {
        handle_create(schema, &keys, &values, connection)?;
    } else {
        handle_insert(schema, &keys, &values, connection)?;
    };

    let response = Response {
        status: Status::Ok,
        message: String::from("Succesfully created"),
    };

    let res_json = serde_json::to_string(&response)?;
    response.send_response(stream, &res_json, DocType::API);

    Ok(())
}

fn handle_insert(
    schema: &str,
    keys: &Vec<String>,
    values: &Vec<String>,
    connection: &Connection,
) -> Result<(), ServerError> {
    let value_query = keys.join(",");
    let place_holder: Vec<String> = (1..=keys.len()).map(|i| format!("?{}", i)).collect();

    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        schema,
        value_query,
        place_holder.join(",")
    );

    connection.execute(&sql, params_from_iter(values))?;
    Ok(())
}

fn handle_create(
    schema: &str,
    keys: &Vec<String>,
    values: &Vec<String>,
    connection: &Connection,
) -> Result<(), ServerError> {
    let (schema, _) = split_by_delimeter(schema, '/')?;

    let mut sql = format!(
        "CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY,",
        schema
    );

    for i in 0..keys.len() {
        let db_type = Type::from_str(&values[i])?;
        sql.push_str(&format!(" {}", db_type.to_sql(&keys[i])));
    }

    // Remove trailing comma and add a parenthese
    sql.pop();
    sql.push(')');

    connection.execute(&sql, ())?;
    Ok(())
}
// Handle PUT Simply update entry

// DELETE delete entry or table

fn handle_get_request(
    stream: &mut TcpStream,
    route: &str,
    doc_type: DocType,
    connection: &Connection,
) -> Result<(), ServerError> {
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
        || absolute_route.starts_with("api");

    if !is_safe {
        let err = "HTTP/1.1 403 Forbidden\r\n\r\n";
        stream.write_all(err.as_bytes()).unwrap();
        return Ok(());
    }

    //  structure like api/* eg api/people or api/games

    if absolute_route.starts_with("api") {
        if absolute_route == "api" || absolute_route == "api/" {
            let mut stmt = connection.prepare(
                "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%';
",
            )?;
            let cols = stmt.query_map([], |row| row.get::<_, String>(0))?;

            let table_names: Vec<String> = cols.filter_map(|c| c.ok()).collect();
            let table_names_json = serde_json::to_string(&table_names)?;

            let response = Response {
                status: Status::Ok,
                message: String::from("Succesfully recieved"),
            };
            response.send_response(stream, &table_names_json, DocType::API);

            return Ok(());
        }

        // Drop the first 4 chars ('api/')
        let mut schema_name = &absolute_route[4..];
        let mut id: Option<&str> = None;

        if let Some((tmp_schema_name, tmp_id)) = schema_name.split_once("/") {
            id = Some(tmp_id);
            schema_name = tmp_schema_name;
        }

        let query = format!("PRAGMA table_info({})", schema_name);
        let mut stmt = connection.prepare(&query)?;

        let columns: Vec<String> = stmt
            .query_map([], |row| row.get(1))?
            .filter_map(|r| r.ok())
            .collect();

        let query = if id.is_some() {
            format!("SELECT * FROM {} WHERE id = ?1", schema_name)
        } else {
            format!("SELECT * FROM {}", schema_name)
        };

        let mut stmt = connection.prepare(&query)?;

        let param = if id.is_some() {
            // Id is guranteed to be valid
            unsafe { params![id.unwrap_unchecked()] }
        } else {
            params![]
        };

        let rows = stmt
            .query_map(param, |row| {
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
            })?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        let rows_json = serde_json::json!(rows).to_string();

        let response = Response {
            status: Status::Ok,
            message: String::from("Succesfully recieved"),
        };
        response.send_response(stream, &rows_json, DocType::API);

        return Ok(());
    }

    let mut buf = String::new();
    let status_header: &str;
    let length: usize;

    let content_type = doc_type.to_string();

    if let Ok(mut file) = File::open(absolute_route) {
        file.read_to_string(&mut buf)?;
        length = buf.len();
        status_header = "HTTP/1.1 200 OK";
    } else {
        let mut file = File::open("static/not-found.html")?;
        file.read_to_string(&mut buf)?;
        length = buf.len();
        status_header = "HTTP/1.1 404 Not Found";
    }

    let response = format!(
        "{status_header}\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\n\r\n{buf}"
    );

    stream.write_all(response.as_bytes()).unwrap();
    Ok(())
}

fn parse_body(
    headers: &[String],
    reader: &mut BufReader<&TcpStream>,
) -> Result<(String, Encoding), ServerError> {
    let content_length = headers
        .iter()
        .find(|c| c.to_lowercase().starts_with("content-length:"))
        .and_then(|c| c.split_once(':'))
        .map(|(_, value)| value.trim().parse::<usize>().unwrap_or_default())
        .ok_or(ServerError::ParseError(
            "Could not find content length".to_string(),
        ))?;
    let content_type = headers
        .iter()
        .find(|c| c.to_lowercase().starts_with("content-type:"))
        .and_then(|c| c.split_once(':'))
        .map(|(_, value)| value.trim())
        .ok_or(ServerError::ParseError(
            "Could not find content type".to_string(),
        ))?;

    let content_type = Encoding::from(content_type);

    let mut body_buffer = vec![0; content_length];
    reader.read_exact(&mut body_buffer)?;

    let body: String = String::from_utf8(body_buffer)
        .map_err(|_| ServerError::ParseError("Could not convert body".to_string()))?;

    Ok((body, Encoding::from(content_type)))
}

fn split_by_delimeter(string: &str, delimiter: char) -> Result<(&str, &str), ServerError> {
    let (first, second) = string
        .split_once(delimiter)
        .ok_or(ServerError::ParseError(format!(
            "Could not split by delimeter: {delimiter}"
        )))?;
    Ok((first, second))
}

fn find_by_pat(string: &str, pat: char) -> Result<usize, ServerError> {
    let idx = string.find(pat).ok_or(ServerError::ParseError(format!(
        "Could not find by pat: {pat}"
    )))?;
    Ok(idx)
}
