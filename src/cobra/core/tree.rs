use std::io;
use std::collections::BTreeMap;
use std::path::Path;
use crate::cobra::core::{
    object::Object,
    object::TreeEntry,
    repository::Repository,
};
use crate::cobra::core::index::IndexEntry;

pub struct Tree {
    entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new() -> Tree {
        Tree {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, name: String, mode: u32, hash: String) {
        self.entries.push(TreeEntry { mode, name, hash });
    }

    pub fn to_object(&self) -> Object {
        Object::Tree(self.entries.clone())
    }
}

/// Builds a tree object from the index
pub fn build_tree_from_index(repo: &Repository) -> io::Result<Object> {
    let mut trees: BTreeMap<String, Tree> = BTreeMap::new();
    trees.insert("".to_string(), Tree::new());

    // First pass: create tree objects for each directory
    for entry in repo.index.entries() {
        let path = Path::new(&entry.path);
        let parent_path = path.parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "".to_string());

        // Ensure parent directory tree exists
        if !trees.contains_key(&parent_path) {
            trees.insert(parent_path.clone(), Tree::new());
        }

        // Add entry to parent tree
        let filename = path.file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?
            .to_string_lossy()
            .into_owned();

        let tree = trees.get_mut(&parent_path).unwrap();
        tree.add_entry(filename, entry.mode, entry.hash.clone());
    }

    // Second pass: build tree objects from bottom up
    let mut root_tree = None;
    let mut tree_hashes = BTreeMap::new();

    // First, create all tree objects and store their hashes
    for (path, tree) in &trees {
        let tree_object = tree.to_object();
        let tree_hash = tree_object.hash();
        tree_object.write_to_objects_dir(&repo.git_dir)?;
        tree_hashes.insert(path.clone(), tree_hash);
    }

    // Then, update parent trees with the hashes
    let mut paths: Vec<String> = trees.keys().cloned().collect();
    paths.sort_by(|a, b| b.len().cmp(&a.len())); // Sort by length descending

    for path in paths {
        if path.is_empty() {
            root_tree = Some(trees[&path].to_object());
        } else {
            let parent_path = Path::new(&path)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "".to_string());

            let name = Path::new(&path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            let tree_hash = tree_hashes[&path].clone();
            let parent_tree = trees.get_mut(&parent_path).unwrap();
            parent_tree.add_entry(name, 0o040000, tree_hash);
        }
    }

    Ok(root_tree.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_build_tree_single_file() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut repo = Repository::init(temp_dir.path().to_str().unwrap())?;

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content")?;

        // Add file to index
        let metadata = fs::metadata(&test_file)?;
        let entry = IndexEntry::new(
            "test.txt".into(),
            "1234567890123456789012345678901234567890".to_string(),
            metadata,
        );
        repo.add_to_index(entry)?;

        // Build tree
        let tree = build_tree_from_index(&repo)?;
        match tree {
            Object::Tree(entries) => {
                assert_eq!(entries.len(), 1);
                let entry = &entries[0];
                assert_eq!(entry.name, "test.txt");
                assert_eq!(entry.mode, 0o100644);
                assert_eq!(entry.hash, "1234567890123456789012345678901234567890");
            }
            _ => panic!("Expected tree object"),
        }

        Ok(())
    }

    #[test]
    fn test_build_tree_nested() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut repo = Repository::init(temp_dir.path().to_str().unwrap())?;

        // Create test files
        fs::create_dir_all(temp_dir.path().join("src"))?;
        let test_file1 = temp_dir.path().join("src/main.rs");
        let test_file2 = temp_dir.path().join("src/lib.rs");
        fs::write(&test_file1, "main content")?;
        fs::write(&test_file2, "lib content")?;

        // Add files to index
        let metadata1 = fs::metadata(&test_file1)?;
        let metadata2 = fs::metadata(&test_file2)?;
        let entry1 = IndexEntry::new(
            "src/main.rs".into(),
            "1111111111111111111111111111111111111111".to_string(),
            metadata1,
        );
        let entry2 = IndexEntry::new(
            "src/lib.rs".into(),
            "2222222222222222222222222222222222222222".to_string(),
            metadata2,
        );
        repo.add_to_index(entry1)?;
        repo.add_to_index(entry2)?;

        // Build tree
        let tree = build_tree_from_index(&repo)?;
        match tree {
            Object::Tree(entries) => {
                assert_eq!(entries.len(), 1);
                let src_entry = &entries[0];
                assert_eq!(src_entry.name, "src");
                assert_eq!(src_entry.mode, 0o040000);

                // Read src tree
                let src_tree = Object::read_from_objects_dir(&repo.git_dir, &src_entry.hash)?;
                match src_tree {
                    Object::Tree(entries) => {
                        assert_eq!(entries.len(), 2);
                        // Sort entries by name to ensure consistent ordering
                        let mut entries = entries;
                        entries.sort_by(|a, b| a.name.cmp(&b.name));
                        
                        let lib_entry = &entries[0];
                        let main_entry = &entries[1];
                        assert_eq!(lib_entry.name, "lib.rs");
                        assert_eq!(lib_entry.mode, 0o100644);
                        assert_eq!(lib_entry.hash, "2222222222222222222222222222222222222222");
                        assert_eq!(main_entry.name, "main.rs");
                        assert_eq!(main_entry.mode, 0o100644);
                        assert_eq!(main_entry.hash, "1111111111111111111111111111111111111111");
                    }
                    _ => panic!("Expected tree object"),
                }
            }
            _ => panic!("Expected tree object"),
        }

        Ok(())
    }
} 