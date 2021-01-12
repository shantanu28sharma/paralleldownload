use crate::error::DownloadError;
use reqwest;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::{SeekFrom, Write as fsWrite};
use std::sync::{Arc, RwLock};
use tokio::{self, sync::mpsc};

pub struct Write {
    file: FileInfo,
    offset: u64,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct FileInfo {
    url: String,
    path: String,
}

#[derive(Debug)]
pub struct ErrorInfo {
    file: FileInfo,
    error: DownloadError,
}

pub struct DownloadInfo {
    success: RwLock<Vec<FileInfo>>,
    errors: RwLock<Vec<ErrorInfo>>,
}

pub struct Downloader {
    count: usize,
    max_conn: usize,
    push_handle: mpsc::UnboundedSender<FileInfo>,
    pub download_info: Arc<DownloadInfo>,
}

impl Clone for FileInfo {
    fn clone(&self) -> FileInfo {
        FileInfo {
            url: self.url.clone(),
            path: self.path.clone(),
        }
    }
}

impl Clone for ErrorInfo {
    fn clone(&self) -> Self {
        ErrorInfo {
            file: self.file.clone(),
            error: DownloadError::EnqueError("This did not complete".to_string())
        }
    }
}

impl DownloadInfo {
    fn new() -> Self {
        Self {
            success: RwLock::new(vec![]),
            errors: RwLock::new(vec![]),
        }
    }
    fn push_success(&self, file: FileInfo) {
        self.success.write().unwrap().push(file);
    }
    fn push_error(&self, file: FileInfo, error: DownloadError) {
        self.errors.write().unwrap().push(ErrorInfo { file, error });
    }
    fn get_success(&self) -> Vec<FileInfo> {
        let mut ret : Vec<FileInfo> = vec![];
        for file in self.success.read().unwrap().iter() {
            ret.push(file.clone());
        }
        ret
    }
    fn get_err(&self) -> Vec<ErrorInfo> {
        let mut ret : Vec<ErrorInfo> = vec![];
        for file in self.errors.read().unwrap().iter() {
            ret.push(file.clone());
        }
        ret
    }
}

impl Downloader {
    pub fn new(max_conn: Option<usize>) -> Self {
        let (sx, rx) = mpsc::unbounded_channel();
        let d_info = Arc::new(DownloadInfo::new());
        tokio::spawn(Self::start_download(rx, d_info.clone()));
        Downloader {
            count: 0,
            max_conn: max_conn.unwrap_or(10),
            push_handle: sx,
            download_info: d_info.clone(),
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
                Err(_) => {
                    let err = format!("Could not send over mpsc channel");
                    Err(DownloadError::EnqueError(err))
                }
            }
        });
        Ok(())
    }

    pub fn get_err(&self) -> Vec<ErrorInfo> {
        self.download_info.get_err()
    }

    pub fn get_success(&self) -> Vec<FileInfo> {
        self.download_info.get_success()
    }

    pub async fn start_download(
        mut reciever: mpsc::UnboundedReceiver<FileInfo>,
        d_info: Arc<DownloadInfo>,
    ) -> Result<(), DownloadError> {
        let (wsx, wrx) = mpsc::unbounded_channel();
        tokio::spawn(Self::writer(wrx, d_info.clone()));
        loop {
            let file = reciever.recv().await.expect("Cannot recieve from queue");
            let write_push_handle = wsx.clone();
            let d_info = d_info.clone();
            let task = async move {
                match Self::ind_down(file.clone(), write_push_handle).await {
                    Ok(_) => d_info.push_success(file),
                    Err(e) => d_info.push_error(file.clone(), e),
                }
            };
            tokio::spawn(task);
        }
    }

    async fn ind_down(
        file: FileInfo,
        writer: mpsc::UnboundedSender<Write>,
    ) -> Result<(), DownloadError> {
        let mut res = reqwest::get(&file.url).await?;
        let mut written = 0;
        while let Some(chunk) = res.chunk().await? {
            let file = file.clone();
            match writer.send(Write {
                file,
                offset: written,
                data: chunk.to_vec(),
            }) {
                Ok(_) => {}
                Err(_) => {
                    let e = DownloadError::EnqueError("Cannot send to writer".to_string());
                    return Err(e);
                }
            }
            written += chunk.len() as u64;
        }

        Ok(())
    }

    pub async fn writer(
        mut reciever: mpsc::UnboundedReceiver<Write>,
        d_info: Arc<DownloadInfo>,
    ) -> Result<(), DownloadError> {
        loop {
            let write = reciever.recv().await;
            let write = write.unwrap();
            let file = write.file.clone();
            let d_info =d_info.clone();
            tokio::task::spawn_blocking(move || match Self::ind_writer(write) {
                Ok(_) => {}
                Err(e) => d_info.push_error(file, e)
            });
        }
    }

    fn ind_writer(write: Write) -> Result<(), DownloadError> {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .open(write.file.path)?;
        f.seek(SeekFrom::Start(write.offset))?;
        f.write(&write.data)?;
        Ok(())
    }
}
