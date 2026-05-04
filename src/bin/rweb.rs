use std::env;
use std::process::Command;

struct Data {
    keys: Vec<String>,
    values: Vec<String>,
}

impl Data {
    fn from_args(args: &Vec<String>) -> Self {
        let mut keys: Vec<String> = Vec::new();
        let mut values: Vec<String> = Vec::new();

        for string in args {
            if let Some((k, v)) = string.split_once("=") {
                keys.push(k.to_string());
                values.push(v.to_string());
            }
        }
        Self { keys, values }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];

    match cmd.as_str() {
        // TODO: use --help for standard convention
        "help" => println!("Usage is rweb <cmd> <db>"),
        "get" => {
            let db = &args[2];
            let mut curl = format!("curl http://localhost:8080/api/{}", db);

            if flag_exists(&args, "--id") {
                let id = &args[4];
                curl.push('/');
                curl.push_str(id);
            }
            exec_curl(&curl);
        }
        "create" => {
            let db = &args[2];
            let mut curl = format!("curl -X POST http://localhost:8080/api/{}/create", db);
            let header = r#" -H "Content-Type: application/json""#;

            let Data { keys, values } = Data::from_args(&args);
            let mut map = serde_json::Map::new();

            for (key, value) in keys.iter().zip(values.iter()) {
                map.insert(key.clone(), serde_json::Value::String(value.clone()));
            }
            let body = serde_json::Value::Object(map).to_string();

            curl.push_str(header);
            curl.push_str(&format!(" -d '{}'", body));

            exec_curl(&curl);
        }
        "insert" => {
            let db = &args[2];
            let mut curl = format!("curl -X POST http://localhost:8080/api/{}", db);
            let header = r#" -H "Content-Type: application/json""#;

            let Data { keys, values } = Data::from_args(&args);
            let mut map = serde_json::Map::new();

            for (key, value) in keys.iter().zip(values.iter()) {
                map.insert(key.to_string(), to_value(&value));
            }
            let body = serde_json::Value::Object(map).to_string();

            curl.push_str(header);
            curl.push_str(&format!(" -d '{}'", body));

            exec_curl(&curl);
        }
        "put" | "patch" => {
            let db = &args[2];
            let id = &args[4];
            let mut curl = format!("curl -X PATCH http://localhost:8080/api/{}/{}", db, id);
            let header = r#" -H "Content-Type: application/json""#;

            let Data { keys, values } = Data::from_args(&args);
            let mut map = serde_json::Map::new();

            for (key, value) in keys.iter().zip(values.iter()) {
                map.insert(key.to_string(), to_value(&value));
            }
            let body = serde_json::Value::Object(map).to_string();

            curl.push_str(header);
            curl.push_str(&format!(" -d '{}'", body));

            exec_curl(&curl);
        }
        "delete" => {
            let db = &args[2];
            let mut curl = format!("curl -X DELETE http://localhost:8080/api/{}", db);

            if flag_exists(&args, "--id") {
                let id = &args[4];
                curl.push('/');
                curl.push_str(id);
            } else {
                curl.push_str("/delete");
            }
            exec_curl(&curl);
        }
        _ => todo!(),
    }
}

fn exec_curl(curl: &str) {
    let output = Command::new("sh")
        .arg("-c")
        .arg(&curl)
        .output()
        .expect("failed to execute process");

    let pp = to_pretty_string(&output.stdout);
    println!("{}", pp);
}

fn to_value(value: &str) -> serde_json::Value {
    let val = if let Ok(i) = value.parse::<i64>() {
        serde_json::Value::Number(i.into())
    } else if let Ok(f) = value.parse::<f64>() {
        serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap())
    } else {
        serde_json::Value::String(value.to_string())
    };
    val
}

fn to_pretty_string(vec: &[u8]) -> String {
    let raw_output = String::from_utf8_lossy(vec);
    let json: serde_json::Value = serde_json::from_str(&raw_output).unwrap();
    let pp = serde_json::to_string_pretty(&json).unwrap();
    pp
}

// Check if the flag exist in the list of args
fn flag_exists(args: &Vec<String>, flag: &str) -> bool {
    args.iter().any(|x| x == flag)
}
