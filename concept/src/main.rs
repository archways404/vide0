use std::fs::File;
use std::io::{self, Read};
use base64::{encode_config, STANDARD};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::task;
use futures::stream::{self, StreamExt};

const SEGMENT_SIZE: usize = 50 * 1024 * 1024; // 50 MB
const CHUNK_SIZE: usize = 5 * 1024 * 1024;    // 5 MB
const MAX_CONCURRENT_TASKS: usize = 5;        // Limit concurrent tasks to control memory usage

#[derive(Serialize, Deserialize)]
struct ChunkResponse {
    title: String,
}

#[derive(Serialize)]
struct Chunks {
    chunks: BTreeMap<String, String>, // Use BTreeMap to maintain order
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Start the timer
    let now = Instant::now();

    // Encode the file and save the chunks in /tmp
    let file_path = "myvideo.mp4"; // Change this to your file's path
    let mut file = File::open(file_path)?;

    let mut buffer = vec![0; SEGMENT_SIZE];
    let mut chunk_counter = 0;

    let client = Client::new();
    let chunks = Arc::new(Mutex::new(BTreeMap::new())); // Use BTreeMap here

    let mut tasks = Vec::new();

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let segment = &buffer[..bytes_read];
        let encoded_segment = encode_config(segment, STANDARD);

        for chunk in encoded_segment.as_bytes().chunks(CHUNK_SIZE) {
            let chunk_file_name = format!("chunk-{}", chunk_counter);
            let chunk_str = String::from_utf8_lossy(chunk).to_string();
            let client = client.clone();
            let chunks = Arc::clone(&chunks);

            let task = task::spawn(async move {
                match post_chunk(&client, &chunk_str).await {
                    Ok(response) => {
                        let title = response.title.replace(" - Ghostbin", "");
                        let mut chunks = chunks.lock().unwrap();
                        println!("Uploaded chunk: {}", chunk_file_name);
                        chunks.insert(chunk_file_name, title);
                    }
                    Err(e) => {
                        eprintln!("Failed to post chunk: {:?}", e);
                    }
                }
            });

            tasks.push(task);
            chunk_counter += 1;
        }
    }

    // Limit the number of concurrent tasks
    stream::iter(tasks)
        .for_each_concurrent(MAX_CONCURRENT_TASKS, |task| async {
            task.await.unwrap();
        })
        .await;

    // Save the chunks as JSON
    let chunks = Arc::try_unwrap(chunks).expect("Arc::try_unwrap failed").into_inner().unwrap();
    let chunks_json = Chunks { chunks };
    let json_data = serde_json::to_string_pretty(&chunks_json).unwrap();

    // Upload the JSON data
    match post_chunk(&client, &json_data).await {
        Ok(response) => {
            let title = response.title.replace(" - Ghostbin", "");
            println!("Uploaded JSON: {}", title);
        }
        Err(e) => {
            eprintln!("Failed to upload JSON: {:?}", e);
        }
    }

    let elapsed = now.elapsed();
    println!("Encoding and saving chunks elapsed time: {:.2?}", elapsed);

    Ok(())
}

async fn post_chunk(client: &Client, chunk: &str) -> Result<ChunkResponse, reqwest::Error> {
    let params = [
        ("lang", "text"),
        ("text", chunk),
        ("expire", "-1"),
        ("password", ""),
        ("title", ""),
    ];

    let response = client
        .post("https://pst.innomi.net/paste/new")
        .form(&params)
        .send()
        .await?
        .text()
        .await?;

    let title_start = response.find("<title>").unwrap() + 7;
    let title_end = response.find(" - Ghostbin</title>").unwrap();
    let title = &response[title_start..title_end];

    Ok(ChunkResponse {
        title: title.to_string(),
    })
}
