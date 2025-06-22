// Working directory interface 
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use walkdir::WalkDir;
use crate::cobra::core::{
    repository::Repository,
    object::Object,
    index::IndexEntry,
};

/// Represents the state of the working directory
#[derive(Debug, Clone)]
pub struct WorkspaceState {
    /// Map of file paths to their content hashes
    pub files: HashMap<PathBuf, String>,
    /// Map of file paths to their metadata
    pub metadata: HashMap<PathBuf, fs::Metadata>,
}

impl WorkspaceState {
    /// Creates a new workspace state by scanning the working directory
    pub fn from_workspace(repo: &Repository) -> io::Result<WorkspaceState> {
        let mut files = HashMap::new();
        let mut metadata = HashMap::new();
        let cobra_dir = repo.root_path.join(".cobra");

        for entry in WalkDir::new(&repo.root_path)
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
                if let Ok(relative_path) = entry.path().strip_prefix(&repo.root_path) {
                    let relative_path = relative_path.to_path_buf();
                    
                    // Read file content and create blob
                    let content = fs::read(entry.path())?;
                    let blob = Object::new_blob(content);
                    let hash = blob.hash();
                    
                    // Store blob in objects directory
                    blob.write_to_objects_dir(&repo.git_dir)?;
                    
                    // Store file info
                    files.insert(relative_path.clone(), hash);
                    metadata.insert(relative_path, fs::metadata(entry.path())?);
                }
            }
        }

        Ok(WorkspaceState { files, metadata })
    }

    /// Creates a tree object from the workspace state
    pub fn create_tree(&self, repo: &Repository) -> io::Result<String> {
        let mut tree_entries = Vec::new();
        
        for (path, hash) in &self.files {
            if let Some(metadata) = self.metadata.get(path) {
                let mode = metadata.mode() as u32;
                let name = path.file_name()
                    .ok_or_else(|| io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Invalid file path",
                    ))?
                    .to_string_lossy()
                    .to_string();
                
                tree_entries.push((name, mode, hash.clone()));
            }
        }
        
        // Sort entries for consistent tree creation
        tree_entries.sort_by(|a, b| a.0.cmp(&b.0));
        
        // Create tree object
        let tree = Object::new_tree_from_entries(tree_entries);
        let tree_hash = tree.hash();
        tree.write_to_objects_dir(&repo.git_dir)?;
        
        Ok(tree_hash)
    }

    /// Applies the workspace state to the working directory
    pub fn apply_to_workspace(&self, repo: &Repository) -> io::Result<()> {
        // First, remove all existing files (except .cobra directory)
        self.clean_workspace(repo)?;
        
        // Then create all files from the state
        for (path, hash) in &self.files {
            let full_path = repo.root_path.join(path);
            
            // Create parent directories
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            // Read blob and write to file
            let blob = Object::read_from_objects_dir(&repo.git_dir, hash)?;
            match blob {
                Object::Blob(content) => {
                    fs::write(&full_path, content)?;
                    
                    // Restore file permissions if we have metadata
                    if let Some(metadata) = self.metadata.get(path) {
                        let mut perms = fs::metadata(&full_path)?.permissions();
                        perms.set_mode(metadata.mode());
                        fs::set_permissions(&full_path, perms)?;
                    }
                }
                _ => return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Workspace state contains non-blob object",
                )),
            }
        }
        
        Ok(())
    }

    /// Cleans the working directory (removes all files except .cobra)
    fn clean_workspace(&self, repo: &Repository) -> io::Result<()> {
        let cobra_dir = repo.root_path.join(".cobra");
        
        for entry in WalkDir::new(&repo.root_path)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| {
                !e.path().starts_with(&cobra_dir) && 
                !e.path().to_string_lossy().contains("/.") &&
                !e.path().file_name().map_or(false, |n| n.to_string_lossy().starts_with("."))
            })
        {
            let entry = entry?;
            if entry.file_type().is_file() {
                fs::remove_file(entry.path())?;
            } else if entry.file_type().is_dir() {
                // Only remove empty directories
                if fs::read_dir(entry.path())?.next().is_none() {
                    fs::remove_dir(entry.path())?;
                }
            }
        }
        
        Ok(())
    }

    /// Checks if there are conflicts between this state and the current workspace
    pub fn check_conflicts(&self, repo: &Repository) -> io::Result<Vec<PathBuf>> {
        let mut conflicts = Vec::new();
        let current_state = WorkspaceState::from_workspace(repo)?;
        
        for (path, hash) in &self.files {
            if let Some(current_hash) = current_state.files.get(path) {
                if current_hash != hash {
                    conflicts.push(path.clone());
                }
            }
        }
        
        Ok(conflicts)
    }
}

/// Represents a complete stash (working directory + index state)
#[derive(Debug, Clone)]
pub struct StashState {
    /// Working directory state
    pub workspace: WorkspaceState,
    /// Index state (staged changes)
    pub index: HashMap<PathBuf, IndexEntry>,
    /// Parent commit hash
    pub parent: String,
    /// Stash message
    pub message: String,
}

impl StashState {
    /// Creates a new stash state from current workspace and index
    pub fn create(repo: &Repository, message: &str) -> io::Result<StashState> {
        let workspace = WorkspaceState::from_workspace(repo)?;
        
        // Get current index state
        let mut index = HashMap::new();
        for entry in repo.index.entries() {
            index.insert(entry.path.clone(), entry.clone());
        }
        
        // Get current HEAD commit
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        let head_content = ref_store.read_head()?
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "HEAD reference not found",
            ))?;

        let parent = if head_content.starts_with("ref: ") {
            let current_branch_ref = &head_content[5..];
            ref_store.read_ref(current_branch_ref)?
                .ok_or_else(|| io::Error::new(
                    io::ErrorKind::NotFound,
                    "Current branch reference not found",
                ))?
        } else {
            head_content
        };

        Ok(StashState {
            workspace,
            index,
            parent,
            message: message.to_string(),
        })
    }

    /// Creates a commit object from the stash state
    pub fn create_commit(&self, repo: &Repository) -> io::Result<String> {
        // Create tree from workspace state
        let tree_hash = self.workspace.create_tree(repo)?;
        
        // Create commit
        let author = crate::cobra::core::signature::Signature::new(
            "Your Name".to_string(),
            "you@example.com".to_string(),
        );
        let committer = author.clone();

        let commit = Object::new_commit(
            tree_hash,
            vec![self.parent.clone()],
            author,
            committer,
            self.message.clone(),
        );

        let commit_hash = commit.hash();
        commit.write_to_objects_dir(&repo.git_dir)?;
        
        Ok(commit_hash)
    }

    /// Applies the stash state to the working directory and index
    pub fn apply(&self, repo: &Repository) -> io::Result<()> {
        // Check for conflicts
        let conflicts = self.workspace.check_conflicts(repo)?;
        if !conflicts.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Conflicts detected: {:?}", conflicts),
            ));
        }
        
        // Apply workspace state
        self.workspace.apply_to_workspace(repo)?;
        
        // Apply index state (this would require updating the repository's index)
        // For now, we'll just note that this needs to be implemented
        
        Ok(())
    }
} 