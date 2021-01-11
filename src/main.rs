use std::error::Error;
use tokio::{self};
mod downloader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut d1 = downloader::Downloader::new(None, None);
    d1.enque_file(
        "http://data.sunpy.org/sample-data/predicted-sunspot-radio-flux.txt".to_string(),
        "./".to_string(),
    );
    
    let starter = d1.start_download().await;
    Ok(())
}
