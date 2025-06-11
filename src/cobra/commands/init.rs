// Initialize new repository
use std::io;
use crate::cobra::core::repository::Repository;

pub fn run(path: &str) -> io::Result<()> {
    Repository::init(path)?;
    println!("Initialized empty Cobra repository in {}", path);
    Ok(())
} 