use std::io;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use walkdir::WalkDir;
use crate::cobra::core::{
    repository::Repository,
    index::IndexEntry,
    object::Object,
    ref_store::RefStore,
};

fn get_workspace_files(repo_root: &Path) -> io::Result<HashSet<PathBuf>> {
    let mut files = HashSet::new();
    let cobra_dir = repo_root.join(".cobra");

    for entry in WalkDir::new(repo_root)
        .min_depth(1)  // Skip root directory
        .into_iter()
        .filter_entry(|e| {
            // Skip .cobra directory and hidden files
            !e.path().starts_with(&cobra_dir) && 
            !e.path().to_string_lossy().contains("/.") &&
            !e.path().file_name().map_or(false, |n| n.to_string_lossy().starts_with("."))
        })
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Ok(path) = entry.path().strip_prefix(repo_root) {
                files.insert(path.to_path_buf());
            }
        }
    }
    Ok(files)
}

fn is_file_modified(repo: &Repository, path: &Path, index_entry: &IndexEntry) -> io::Result<bool> {
    let full_path = repo.root_path.join(path);
    let metadata = fs::metadata(&full_path)?;
    
    println!("Checking file: {}", path.display());
    println!("  Current size: {}, Index size: {}", metadata.len(), index_entry.size);
    println!("  Current mtime: {}, Index mtime: {}", metadata.mtime(), index_entry.mtime);
    
    // Quick check: if mtime and size match, assume content is the same
    if metadata.len() == index_entry.size && 
       metadata.mtime() as u64 == index_entry.mtime {
        return Ok(false);
    }

    // Content check: hash the current file and compare with index
    let content = fs::read(&full_path)?;
    let blob = Object::new_blob(content);
    let current_hash = blob.hash();
    println!("  Current hash: {}, Index hash: {}", current_hash, index_entry.hash);
    Ok(current_hash != index_entry.hash)
}

pub fn run() -> io::Result<()> {
    // Open repository
    let repo = Repository::open(".")?;
    let _ref_store = RefStore::new(repo.git_dir.clone());

    // Get all files in workspace
    let workspace_files = get_workspace_files(&repo.root_path)?;
    
    // Get all files in index
    let index_files: HashSet<_> = repo.index.entries()
        .map(|entry| entry.path.clone())
        .collect();

    // Find untracked files (in workspace but not in index)
    let mut untracked: Vec<_> = workspace_files.difference(&index_files)
        .collect();
    untracked.sort(); // Sort for consistent output

    // Find modified files (in both but content differs)
    let mut modified = Vec::new();
    for path in workspace_files.intersection(&index_files) {
        if let Some(index_entry) = repo.index.entries().find(|e| e.path == *path) {
            if is_file_modified(&repo, path, index_entry)? {
                modified.push(path);
            }
        }
    }
    modified.sort(); // Sort for consistent output

    // Print status
    if !modified.is_empty() {
        println!("Changes not staged for commit:");
        println!("  (use \"cobra add <file>...\" to update what will be committed)");
        for path in &modified {
            println!("\tmodified:   {}", path.display());
        }
        println!();
    }

    if !untracked.is_empty() {
        println!("Untracked files:");
        println!("  (use \"cobra add <file>...\" to include in what will be committed)");
        for path in &untracked {
            println!("\t{}", path.display());
        }
        println!();
    }

    if modified.is_empty() && untracked.is_empty() {
        println!("nothing to commit, working tree clean");
    }

    Ok(())
} 