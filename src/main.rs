use anyhow::{Ok, Result};
use reqwest::StatusCode;
use reqwest::{
    header::{CONTENT_LENGTH, RANGE},
    Client,
};
use std::fs;
use std::io::BufRead;
use std::path::Path;
use std::str::FromStr;
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Write},
};

use tokio;
#[tokio::main]
async fn main() -> Result<()> {
    download().await?;
    Ok(())
}

fn merge_file(target: String, num: i64) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .read(true)
        .write(true)
        .create(true)
        .open(target)?;
    for i in 0..num {
        let path = format!("./{}.tar.gz.temp", i);
        let open = File::open(Path::new(&path))?;
        let mut reader = BufReader::with_capacity(1024, open);
        loop {
            let buffer = reader.fill_buf()?;
            let buff_len = buffer.len();
            if buff_len == 0 {
                break;
            }
            file.write(buffer)?;
            reader.consume(buff_len);
        }
        let _ = fs::remove_file(path);
    }
    Ok(())
}

async fn download_file(start: i64, end: i64, num: i64, url: &str) -> Result<()> {
    let temp_path = format!("./{}.tar.gz.temp", num);
    let mut output_file = File::create(temp_path)?;
    let range = format!("bytes={}-{}", start, end);
    let response = Client::new().get(url).header(RANGE, range).send().await?;

    let status = response.status();
    if !(status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT) {
        println!("{}", status)
    }
    let x = response.bytes().await?;
    let z = x.to_vec();
    output_file.write_all(&z)?;
    Ok(())
}

async fn download() -> Result<(), anyhow::Error> {
    let target_path = "./temp.tar.gz";
    let url = "https://download.oracle.com/java/20/latest/jdk-20_linux-aarch64_bin.tar.gz";
    const CHUNK_SIZE: i64 = 1024 * 1024 * 15;

    let response = Client::new().head(url).send().await?;
    let length = response
        .headers()
        .get(CONTENT_LENGTH)
        .ok_or("response doesn't include the content length")
        .unwrap();
    let length = i64::from_str(length.to_str()?)
        .map_err(|_| "invalid Content-Length header")
        .unwrap();
    println!("starting download...");
    let num = (length / CHUNK_SIZE) + 1;
    let mut start = 0;
    let mut ts = vec![tokio::spawn(async move {
        let _ = download_file(start, start + CHUNK_SIZE, 0, url).await;
    })];
    for i in 1..num {
        start = i * CHUNK_SIZE + i;
        let t = tokio::spawn(async move {
            let _ = download_file(start, start + CHUNK_SIZE, i, url).await;
        });
        ts.push(t);
    }
    for thread in ts {
        let _ = thread.await;
    }
    let _ = merge_file(target_path.to_string(), num);
    println!("Finished with success!");
    Ok(())
}
