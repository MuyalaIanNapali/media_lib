//! Flat-memory benchmark helper: strip+encrypt a file through the pipeline.
//! Usage: bench_strip_encrypt <mime> <input_path> <output_path>
use std::env;
use std::fs::File;

use media_lib::crypto::keys::generate_key;
use media_lib::pipeline::strip_and_encrypt;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mime = &args[1];
    let input = File::open(&args[2]).expect("open input");
    // The AES-CTR path rewrites its header at the end, so the output must be
    // seekable; a plain File is.
    let output = File::create(&args[3]).expect("create output");

    let key = generate_key();
    strip_and_encrypt(mime, &key, input, output).expect("pipeline");
    eprintln!("done");
}
