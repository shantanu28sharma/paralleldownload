use reqwest;
use std::fmt;
use std::io;

#[derive(Debug, Clone)]
pub enum DownloadError {
    HttpError(String),
    WriteError(String),
    EnqueError(String),
}

impl std::error::Error for DownloadError {}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DownloadError::HttpError(e) => write!(f, "{:#?}", e),
            DownloadError::WriteError(e) => write!(f, "{:#?}", e),
            DownloadError::EnqueError(e) => write!(f, "{}", e),
        }
    }
}

impl From<reqwest::Error> for DownloadError {
    fn from(err: reqwest::Error) -> Self {
        DownloadError::HttpError(format!("HTTP Error {:?}", err))
    }
}

impl From<io::Error> for DownloadError {
    fn from(err: io::Error) -> Self {
        DownloadError::WriteError(format!("I/O Error {:?}", err))
    }
}
