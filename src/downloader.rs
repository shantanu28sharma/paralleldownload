use reqwest;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{self, SeekFrom, Write as fsWrite};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::{self, sync::mpsc};

#[derive(Debug)]
enum Error_ {
    LIMITREACHED,
}

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

    pub fn enque_file(&mut self, url: String, path: String) {
        if self.count == self.max_conn {
            eprintln!("limit reached");
        }
        self.count += 1;
        let sx = self.push_handle.clone();
        tokio::spawn(async move {
            sx.send(FileInfo {
                url: url,
                path: path,
            });
        });
    }

    pub async fn start_download(&mut self) {
        let (wsx, wrx) = mpsc::unbounded_channel();
        tokio::spawn(Self::writer(wrx));
        loop {
            let file = self.take_handle.recv().await.unwrap();
            let write_push_handle = wsx.clone();
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
            writer.send(Write {
                file: file.clone(),
                offset: written,
                data: chunk.to_vec(),
            });
            written += chunk.len() as u64;
        }
        Ok(())
    }

    pub async fn writer(mut reciever: mpsc::UnboundedReceiver<Write>) {
        loop {
            let write = reciever.recv().await;
            tokio::task::spawn_blocking(|| Self::ind_writer(write.unwrap()));
        }
    }

    fn ind_writer(write: Write) -> io::Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .open(write.file)?;
        match f.seek(SeekFrom::Start(write.offset)){
            Err(e)=>println!("{}", e),
            Ok(_)=>{}
        }
        match f.write(&write.data){
            Err(e)=>println!("{}", e),
            Ok(_)=>{}
        }
        Ok(())
    }
}
