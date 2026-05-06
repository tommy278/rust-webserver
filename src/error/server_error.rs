#[derive(Debug)]
pub enum ServerError {
    DbError(rusqlite::Error),
    ParseError(String),
    IoError(std::io::Error),
    JsonError(serde_json::Error),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::DbError(e) => write!(f, "DB error: {e}"),
            ServerError::ParseError(e) => write!(f, "Parse Error: {e}"),
            ServerError::IoError(e) => write!(f, "I/O Error: {e}"),
            ServerError::JsonError(e) => write!(f, "Json Error: {e}"),
        }
    }
}

impl std::error::Error for ServerError {}

impl From<rusqlite::Error> for ServerError {
    fn from(e: rusqlite::Error) -> Self {
        ServerError::DbError(e)
    }
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        ServerError::IoError(e)
    }
}

impl From<String> for ServerError {
    fn from(e: String) -> Self {
        ServerError::ParseError(e)
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(e: serde_json::Error) -> Self {
        ServerError::JsonError(e)
    }
}
