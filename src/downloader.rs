use reqwest;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, SeekFrom, Write as fsWrite};
use tokio::{self, sync::mpsc};
use std::sync::Arc;

#[derive(Debug)]
enum Error_ {
    LIMITREACHED,
}

struct Write {
    file: String,
    offset: u64,
    data: Vec<u8>,
}

struct FileInfo {
    url: String,
    path: String
}

pub struct Downloader {
    count: usize,
    max_conn: usize,
    push_handle: mpsc::UnboundedSender<FileInfo>,
    take_handle: mpsc::UnboundedReceiver<FileInfo>,
    write_push_handle: mpsc::UnboundedSender<Write>,
    write_take_handle: mpsc::UnboundedReceiver<Write>,
}

impl Downloader {
    pub fn new(max_conn: Option<usize>, overwrite: Option<bool>) -> Self {
        let (sx, rx) = mpsc::unbounded_channel();
        let (wsx, wrx) = mpsc::unbounded_channel();
        Downloader {
            count: 0,
            max_conn: max_conn.unwrap_or(10),
            push_handle: sx,
            take_handle: rx,
            write_push_handle: wsx,
            write_take_handle: wrx,
        }
    }

    pub fn enque_file(&mut self, url: String, path: String) {
        if self.count == self.max_conn {
            eprintln!("limit reached");
        }
        self.count += 1;
        let sx = self.push_handle.clone();
        tokio::spawn(async move {
            sx.send(FileInfo{
                url: url, 
                path: path
            });
        });
    }

    pub async fn start_download(&mut self) {
        loop {
            let write_push_handle = self.write_push_handle.clone();
            let file = self.take_handle.recv().await.unwrap();
            let task = async move { Self::ind_down(file.url, file.path, write_push_handle).await };
            tokio::spawn(task);
        }
    }

    async fn ind_down(
        url: String,
        file: String,
        writer: mpsc::UnboundedSender<Write>,
    ) -> Result<(), reqwest::Error> {
        let mut res = reqwest::get(&url).await?;
        let mut written = 0;
        while let Some(chunk) = res.chunk().await? {
            writer.send(Write{
                file: file.clone(),
                offset: written,
                data: chunk.to_vec()
            });
            written+=chunk.len() as u64;
        }
        Ok(())
    }

    pub async fn writer(&mut self) {
        loop{
            let write = self.write_take_handle.recv().await.unwrap();
            tokio::task::spawn_blocking(|| Self::ind_writer(write));
        }
    }

    fn ind_writer(write: Write) -> io::Result<()> {
        let mut f = File::open(write.file)?;
        f.seek(SeekFrom::Start(write.offset))?;
        f.write(&write.data)?;
        Ok(())
    }
}
