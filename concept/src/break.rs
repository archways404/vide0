use std::fs::{self, File};
use std::io::{self, Read, Write};
use base64::{encode_config, decode_config, STANDARD};
use std::time::Instant;

const SEGMENT_SIZE: usize = 50 * 1024 * 1024; // 50 MB
const CHUNK_SIZE: usize = 5 * 1024 * 1024;    // 5 MB

fn main() -> io::Result<()> {
    // Start the timer
    let now = Instant::now();
    
    // Encode the file and save the chunks in /tmp
    let file_path = "myvideo.mp4"; // Change this to your file's path
    let mut file = File::open(file_path)?;

    let mut buffer = vec![0; SEGMENT_SIZE];
    let mut chunk_counter = 0;

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let segment = &buffer[..bytes_read];
        let encoded_segment = encode_config(segment, STANDARD);

        for chunk in encoded_segment.as_bytes().chunks(CHUNK_SIZE) {
            let chunk_file_name = format!("tmp/chunk-{}.txt", chunk_counter);
            let mut chunk_file = File::create(&chunk_file_name)?;
            chunk_file.write_all(chunk)?;
            chunk_counter += 1;
        }
    }

    let elapsed = now.elapsed();
    println!("Encoding and saving chunks elapsed time: {:.2?}", elapsed);

    // Loop through the /tmp folder and print out the chunk file names
    println!("\nChunk files in tmp:");
    for entry in fs::read_dir("tmp")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    if file_name_str.starts_with("chunk-") && file_name_str.ends_with(".txt") {
                        println!("{}", file_name_str);
                    }
                }
            }
        }
    }

    // Decode one of the chunks and save it as a new file
    let chunk_to_decode = "tmp/chunk-0.txt"; // Change this to the chunk you want to decode
    let decoded_file_path = "decoded_chunk.mp4";
    decode_chunk_to_file(chunk_to_decode, decoded_file_path)?;

    Ok(())
}

fn decode_chunk_to_file(input_path: &str, output_path: &str) -> io::Result<()> {
    // Read the chunk file
    let mut chunk_file = File::open(input_path)?;
    let mut encoded_data = String::new();
    chunk_file.read_to_string(&mut encoded_data)?;

    // Decode the Base64 data
    let decoded_data = decode_config(&encoded_data, STANDARD).unwrap();

    // Write the decoded data to a new file
    let mut output_file = File::create(output_path)?;
    output_file.write_all(&decoded_data)?;

    println!("Decoded chunk saved to {}", output_path);

    Ok(())
}
