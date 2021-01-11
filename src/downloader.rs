use std::error::Error;
use tokio::{self, net::TcpStream, sync::mpsc};
use reqwest;

#[derive(Debug)]
enum Error_ {
    LIMITREACHED,
}

pub struct Downloader {
    count: usize,
    max_conn: usize,
    push_handle: mpsc::UnboundedSender<String>,
    take_handle: mpsc::UnboundedReceiver<String>,
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
            sx.send(url).unwrap();
        });
    }

    pub async fn start_download(&mut self) {
        loop{
            let url = self.take_handle.recv().await.unwrap();
            let task = async move { Self::ind_down(url).await };
            tokio::spawn(task);
        }
    }

    async fn ind_down(url: String) -> Result<(), reqwest::Error> {
        let mut res = reqwest::get(&url).await?;
        while let Some(chunk) = res.chunk().await? {
            println!("Chunk: {:?}", String::from_utf8_lossy(&chunk));
        }
        Ok(())
    }
}
