#[derive(Debug)]
pub enum CliError {
    NotEnoughArguments,
    IoError(std::io::Error),
    CommandNotFound,
    ParseError(String),
    JsonError(serde_json::Error),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::NotEnoughArguments => write!(f, "Not enough arguments"),
            CliError::IoError(e) => write!(f, "I/O Error: {e}"),
            CliError::CommandNotFound => write!(
                f,
                "Command not found, use 'rweb help' for more information "
            ),
            CliError::ParseError(e) => write!(f, "Parse Error: {e}"),
            CliError::JsonError(e) => write!(f, "Json Error: {e}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::IoError(e)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        CliError::JsonError(e)
    }
}

impl From<String> for CliError {
    fn from(e: String) -> Self {
        CliError::ParseError(e)
    }
}
