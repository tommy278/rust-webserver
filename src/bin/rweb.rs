use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];

    match cmd.as_str() {
        // TODO: use --help for standard convention
        "help" => println!("Usage is rweb <cmd> <db>"),
        "get" => {
            let mut curl = String::from("curl http://localhost:8080/api/");
            let db = &args[2];
            curl.push_str(db);

            if flag_exists(&args, "--id") {
                let id = &args[4];
                curl.push('/');
                curl.push_str(id);
            }
            let output = Command::new("sh")
                .arg("-c")
                .arg(&curl)
                .output()
                .expect("failed to execute process");

            let pp = to_pretty_string(&output.stdout);
            println!("{}", pp);
        }
        "create" => {
            if args.len() > 3 {
                eprintln!("Not enough arguments");
            }
            // TODO: Add functionality
        }
        _ => todo!(),
    }
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
