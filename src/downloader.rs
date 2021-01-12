use reqwest;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::{SeekFrom, Write as fsWrite};
use tokio::{self, sync::mpsc};
// mod error::DownloadError;
use crate::error;
use error::DownloadError;

pub struct Write {
    file: String,
    offset: u64,
    data: Vec<u8>,
}

struct FileInfo {
    url: String,
    path: String,
}

pub struct Downloader {
    count: usize,
    max_conn: usize,
    push_handle: mpsc::UnboundedSender<FileInfo>,
    take_handle: mpsc::UnboundedReceiver<FileInfo>,
}

impl Downloader {
    pub fn new(max_conn: Option<usize>, overwrite: Option<bool>) -> Self {
        let (sx, rx) = mpsc::unbounded_channel();
        Downloader {
            count: 0,
            max_conn: max_conn.unwrap_or(10),
            push_handle: sx,
            take_handle: rx,
        }
    }

    pub fn enque_file(&mut self, url: String, path: String) -> Result<(), DownloadError> {
        if self.count == self.max_conn {
            eprintln!("limit reached");
            return Err(DownloadError::EnqueError("Limit Reached".to_string()));
        }
        self.count += 1;
        let sx = self.push_handle.clone();
        tokio::spawn(async move {
            match sx.send(FileInfo {
                url: url,
                path: path,
            }) {
                Ok(_) => Ok(()),
                Err(e) => {
                    let err = format!("Could not send over mpsc channel");
                    Err(DownloadError::EnqueError(err))
                }
            }
        });
        Ok(())
    }

    pub async fn start_download(&mut self) -> Result<(), DownloadError>{
        let (wsx, wrx) = mpsc::unbounded_channel();
        tokio::spawn(Self::writer(wrx));
        loop {
            let file = self.take_handle.recv().await.expect("Cannot recieve from queue");
            let write_push_handle = wsx.clone();
            let task = async move { Self::ind_down(file.url, file.path, write_push_handle).await };
            tokio::spawn(task);
        }
    }

    async fn ind_down(
        url: String,
        file: String,
        writer: mpsc::UnboundedSender<Write>,
    ) -> Result<(), DownloadError> {
        let mut res = reqwest::get(&url).await?;
        let mut written = 0;
        while let Some(chunk) = res.chunk().await? {
            match writer.send(Write {
                file: file.clone(),
                offset: written,
                data: chunk.to_vec(),
            }) {
                Ok(_) => {}
                Err(_) => {
                    DownloadError::EnqueError("Cannot send to writer".to_string());
                }
            }
            written += chunk.len() as u64;
        }
        Ok(())
    }

    pub async fn writer(mut reciever: mpsc::UnboundedReceiver<Write>) -> Result<(), DownloadError> {
        loop {
            let write = reciever.recv().await;
            tokio::task::spawn_blocking(|| Self::ind_writer(write.unwrap()));
        }
    }

    fn ind_writer(write: Write) -> Result<(), DownloadError> {
        let mut f = OpenOptions::new().write(true).create(true).open(write.file)?;
        f.seek(SeekFrom::Start(write.offset))?;
        f.write(&write.data)?;
        Ok(())
    }
}
