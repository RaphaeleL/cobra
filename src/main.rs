mod cobra;
use std::process;

fn main() {
    if let Err(e) = cobra::cli::run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
