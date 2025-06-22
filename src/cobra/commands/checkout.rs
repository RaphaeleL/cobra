// Switch branches or restore files 
use std::io;
use std::path::Path;
use crate::cobra::core::repository::Repository;

pub fn run(path: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
    
    // Check if this is a branch name
    let branch_ref = format!("refs/heads/{}", path);
    if let Some(_) = ref_store.read_ref(&branch_ref)? {
        // It's a branch, switch to it
        ref_store.update_head(&format!("ref: {}", branch_ref))?;
        println!("Switched to branch '{}'", path);
        return Ok(());
    }
    
    // Check if it's a file path in the index
    let file_path = Path::new(path);
    if let Some(entry) = repo.index.get_entry(file_path) {
        // Restore file from index
        restore_file_from_index(&repo, entry)?;
        println!("Restored '{}' from index", path);
        return Ok(());
    }
    
    // Neither a branch nor a file in index
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("'{}' is neither a branch nor a file in the index", path),
    ))
}

fn restore_file_from_index(repo: &Repository, entry: &crate::cobra::core::index::IndexEntry) -> io::Result<()> {
    // Read the blob object
    let blob = crate::cobra::core::object::Object::read_from_objects_dir(&repo.git_dir, &entry.hash)?;
    
    // Extract blob content
    let content = match blob {
        crate::cobra::core::object::Object::Blob(data) => data,
        _ => return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Index entry does not point to a blob",
        )),
    };
    
    // Write content to file
    let file_path = repo.root_path.join(&entry.path);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file_path, content)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_checkout_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Create a branch
        ref_store.create_branch("test_branch")?;
        
        // Test branch checkout logic directly
        let branch_ref = "refs/heads/test_branch";
        if let Some(_) = ref_store.read_ref(branch_ref)? {
            ref_store.update_head(&format!("ref: {}", branch_ref))?;
        }
        
        // Verify HEAD points to the branch
        let head = ref_store.read_head()?.unwrap();
        assert_eq!(head, "ref: refs/heads/test_branch");
        
        Ok(())
    }

    #[test]
    fn test_checkout_nonexistent_branch() {
        let temp_dir = TempDir::new().unwrap();
        let _repo = Repository::init(temp_dir.path().to_str().unwrap()).unwrap();
        
        // Test non-existent branch logic directly
        let ref_store = crate::cobra::core::ref_store::RefStore::new(temp_dir.path().join(".cobra"));
        let branch_ref = "refs/heads/nonexistent";
        let exists = ref_store.read_ref(branch_ref).unwrap().is_some();
        assert!(!exists);
    }

    #[test]
    fn test_checkout_file_from_index() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        
        // Create a file and add it to index
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content")?;
        
        // Add to index (simplified - in real implementation you'd use the add command)
        let content = fs::read(&file_path)?;
        let blob = crate::cobra::core::object::Object::new_blob(content);
        let hash = blob.hash();
        blob.write_to_objects_dir(&repo.git_dir)?;
        
        // Create index entry
        let entry = crate::cobra::core::index::IndexEntry::new(
            std::path::PathBuf::from("test.txt"),
            hash,
            fs::metadata(&file_path)?,
        );
        repo.add_to_index(entry)?;
        
        // Delete the file
        fs::remove_file(&file_path)?;
        
        // Test file checkout logic directly
        let file_path = Path::new("test.txt");
        if let Some(entry) = repo.index.get_entry(file_path) {
            restore_file_from_index(&repo, entry)?;
        }
        
        // Verify file was restored
        let restored_path = temp_dir.path().join("test.txt");
        assert!(restored_path.exists());
        let content = fs::read_to_string(&restored_path)?;
        assert_eq!(content, "test content");
        
        Ok(())
    }
} 