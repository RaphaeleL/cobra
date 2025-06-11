use std::fs;
use std::io;
use std::path::Path;
use crate::cobra::core::{
    repository::Repository,
    object::Object,
    index::IndexEntry,
};

pub fn run(path: &str) -> io::Result<()> {
    let mut repo = Repository::open(".")?;
    let file_path = Path::new(path);

    // Convert to absolute path if relative
    let absolute_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        repo.root_path.join(file_path)
    };

    // Read file content
    let content = fs::read(&absolute_path)?;
    let metadata = fs::metadata(&absolute_path)?;

    // Create blob object
    let blob = Object::new_blob(content);
    let hash = blob.hash();
    blob.write_to_objects_dir(&repo.git_dir)?;

    // Create index entry with relative path
    let relative_path = if file_path.is_absolute() {
        file_path.strip_prefix(&repo.root_path)
            .map_err(|_| io::Error::new(
                io::ErrorKind::InvalidInput,
                "Path must be inside repository",
            ))?
            .to_path_buf()
    } else {
        file_path.to_path_buf()
    };

    let entry = IndexEntry::new(relative_path, hash, metadata);
    repo.add_to_index(entry)?;

    Ok(())
} 