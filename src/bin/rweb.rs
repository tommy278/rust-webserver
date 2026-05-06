use http::error::rweb::CliError;
use std::env;
use std::process::Command;

struct Data {
    keys: Vec<String>,
    values: Vec<String>,
}

impl Data {
    fn from_args(args: &[String]) -> Self {
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
    exec_cmd(&args);
}

fn exec_cmd(args: &[String]) {
    if let Err(e) = try_exec_cmd(args) {
        eprint!("{e}");
    }
}

fn try_exec_cmd(args: &[String]) -> Result<(), CliError> {
    let cmd = &args[1];

    match cmd.as_str() {
        "help" => {
            let help = r#"
rweb <command> <table> [options]

Commands:
  get <table>                     Get all rows from table
  get <table> --id <id>           Get a single row by id
  get tables                      List all tables
  create <table> <col=type>       Create table
  insert <table> <col=val>        Insert a row into table
  put <table> --id <id> <col=val> Update a row by id
  delete <table>                  Drop the table
  delete <table> --id <id>        Delete a row by id

Examples:
  rweb get games
  rweb get games --id 1
  rweb create games title=text rating=integer
  rweb insert games title=Roblox rating=7
  rweb put games --id 1 rating=10
  rweb delete games --id 1
  rweb delete games
"#;
            println!("{help}")
        }
        "get" => {
            if args.len() < 3 {
                return Err(CliError::NotEnoughArguments);
            }
            let mut curl = String::from("curl http://localhost:8080/api/");
            let db = &args[2];

            if db == "tables" {
                // Simply return a blank api request, server handles logic
                return Ok(exec_curl(&curl)?);
            }

            curl.push_str(db);

            if flag_exists(&args, "--id") {
                let id = &args[4];
                curl.push('/');
                curl.push_str(id);
            }

            exec_curl(&curl)?;
        }
        "create" => {
            if args.len() < 3 {
                return Err(CliError::NotEnoughArguments);
            }

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

            exec_curl(&curl)?;
        }
        "insert" => {
            if args.len() < 3 {
                return Err(CliError::NotEnoughArguments);
            }
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

            exec_curl(&curl)?;
        }
        "put" | "patch" => {
            if args.len() < 5 {
                return Err(CliError::NotEnoughArguments);
            }

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

            exec_curl(&curl)?;
        }
        "delete" => {
            if args.len() < 3 {
                return Err(CliError::NotEnoughArguments);
            }

            let db = &args[2];
            let mut curl = format!("curl -X DELETE http://localhost:8080/api/{}", db);

            if flag_exists(&args, "--id") {
                let id = &args[4];
                curl.push('/');
                curl.push_str(id);
            } else {
                curl.push_str("/delete");
            }
            exec_curl(&curl)?;
        }
        _ => return Err(CliError::CommandNotFound),
    }
    Ok(())
}

fn exec_curl(curl: &str) -> Result<(), CliError> {
    let output = Command::new("sh").arg("-c").arg(&curl).output()?;

    let pp = to_pretty_string(&output.stdout)?;
    println!("{}", pp);

    Ok(())
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

fn to_pretty_string(vec: &[u8]) -> Result<String, CliError> {
    let raw_output = String::from_utf8_lossy(vec);
    let json: serde_json::Value = serde_json::from_str(&raw_output)?;
    let pp = serde_json::to_string_pretty(&json)?;
    Ok(pp)
}

// Check if the flag exist in the list of args
fn flag_exists(args: &[String], flag: &str) -> bool {
    args.iter().any(|x| x == flag)
}
