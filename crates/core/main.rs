use std::cmp::min;
use std::io::{prelude::*, SeekFrom};
use std::fs::File;
use std::path::PathBuf;
use std::str;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use clap::Parser;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    path: PathBuf
}

trait Scanner {
    fn scan(self) -> Result<()>;
}

struct PNGScanner<'a> {
    path: &'a PathBuf,
    output_counter: Arc<AtomicU32>
}

impl<'a> PNGScanner<'a> {
    pub fn new(path: &PathBuf) -> PNGScanner {
        PNGScanner {
            path: path,
            output_counter: Arc::new(AtomicU32::new(0))
        }
    }
}

impl<'a> Scanner for PNGScanner<'a> {
    fn scan(self) -> Result<()> {
        let mut file = File::open(self.path)?;
        loop {
            let mut header = [0; 8];

            let read = file.read(&mut header[..])?;

            if read == 0 {
                break;
            }

            match header {
                [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a] => {
                    println!("Detected PNG header");

                    let file_number = self.output_counter.fetch_add(1, Ordering::SeqCst);
                    let mut output_path = PathBuf::from("output");
                    output_path.push(format!("{:09}.png", file_number));
                    let mut output_file = File::create(output_path)?;

                    output_file.write(&header)?;

                    loop {
                        let mut length_buffer = [0; 4];
                        let mut type_buffer = [0; 4];

                        file.read(&mut length_buffer)?;
                        file.read(&mut type_buffer)?;

                        let mut chunk_length = u32::from_be_bytes(length_buffer);
                        let chunk_type = str::from_utf8(&type_buffer).unwrap();

                        match chunk_type {
                            "IHDR" | "IDAT" | "PLTE" | "tRNS" | "IEND" => {
                                output_file.write(&length_buffer)?;
                                output_file.write(&type_buffer)?;

                                loop {
                                    let bytes_to_read = min(chunk_length, 4096);
                                    let mut data_buffer = vec![0; bytes_to_read.try_into().unwrap()];
                                    file.read(&mut data_buffer)?;
                                    output_file.write(&data_buffer)?;

                                    chunk_length -= bytes_to_read;

                                    if chunk_length == 0 {
                                        break;
                                    }
                                }

                                file.seek(SeekFrom::Current(chunk_length.into()))?;

                                let mut crc_buffer = [0; 4];
                                file.read(&mut crc_buffer)?;
                                output_file.write(&crc_buffer)?;

                                if chunk_type == "IEND" {
                                    break;
                                }
                            },
                            _ => break
                        }
                    }
                },
                _ => {
                    file.seek(SeekFrom::Current(4088))?;
                }
            }
        }

        Ok(())
    }
}

fn main() {
    let args = Args::parse();

    let scanner = PNGScanner::new(&args.path);

    scanner.scan().unwrap();
}
