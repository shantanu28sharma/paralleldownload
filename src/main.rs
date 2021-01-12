use std::error::Error;
use tokio::{self};
mod downloader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut d1 = downloader::Downloader::new(None, None);
    d1.enque_file(
        "http://212.183.159.230/5MB.zip".to_string(),
        "./5mb.zip".to_string(),
    );
    // let writer = downloader::Downloader::writer(d1.write_take_handle);
    d1.enque_file(
        "http://212.183.159.230/10MB.zip".to_string(),
        "./10mb.zip".to_string(),
    );
    d1.start_download().await;
    Ok(())
}
