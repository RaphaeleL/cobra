// Stash management commands
use std::io;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use crate::cobra::core::repository::Repository;

pub fn push(message: Option<&String>) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
    
    let stash_hash = ref_store.create_stash(message.map(|s| s.as_str()))?;
    println!("Saved working directory and index state WIP on current branch: {}", &stash_hash[..7]);
    
    Ok(())
}

pub fn list() -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
    
    let stashes = ref_store.list_stashes()?;
    
    if stashes.is_empty() {
        println!("No stashes found");
        return Ok(());
    }
    
    for (stash_ref, hash) in stashes {
        // Try to get the stash commit to show the message
        if let Ok(Some(stash_commit)) = ref_store.get_stash(&stash_ref) {
            if let Ok(commit_obj) = crate::cobra::core::object::Object::read_from_objects_dir(&repo.git_dir, &stash_commit) {
                if let crate::cobra::core::object::Object::Commit { message, .. } = commit_obj {
                    println!("{}: {}", stash_ref, message.lines().next().unwrap_or(""));
                }
            }
        } else {
            println!("{}: {}", stash_ref, &hash[..7]);
        }
    }
    
    Ok(())
}

pub fn show(stash_ref: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
    
    let stash_hash = ref_store.get_stash(stash_ref)?
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::NotFound,
            format!("Stash '{}' does not exist", stash_ref),
        ))?;
    
    // Read and display the stash commit
    let stash_commit = crate::cobra::core::object::Object::read_from_objects_dir(&repo.git_dir, &stash_hash)?;
    
    match stash_commit {
        crate::cobra::core::object::Object::Commit { tree, parents, author, committer, message } => {
            println!("commit {}", stash_hash);
            println!("Author: {}", author.format());
            println!("Date:   {}", committer.format());
            println!();
            println!("{}", message);
            println!();
            
            // Show the actual diff by comparing with parent
            if let Some(parent_hash) = parents.first() {
                show_diff(&repo, parent_hash, &tree)?;
            }
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Stash does not point to a commit",
            ));
        }
    }
    
    Ok(())
}

pub fn apply(stash_ref: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
    
    let stash_hash = ref_store.get_stash(stash_ref)?
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::NotFound,
            format!("Stash '{}' does not exist", stash_ref),
        ))?;
    
    // Read the stash commit
    let stash_commit = crate::cobra::core::object::Object::read_from_objects_dir(&repo.git_dir, &stash_hash)?;
    
    match stash_commit {
        crate::cobra::core::object::Object::Commit { tree, .. } => {
            // Read the stash tree
            let stash_tree_obj = crate::cobra::core::object::Object::read_from_objects_dir(&repo.git_dir, &tree)?;
            
            match stash_tree_obj {
                crate::cobra::core::object::Object::Tree(entries) => {
                    // Create a workspace state from the stash tree
                    let mut workspace_state = crate::cobra::core::workspace::WorkspaceState {
                        files: std::collections::HashMap::new(),
                        metadata: std::collections::HashMap::new(),
                    };
                    
                    // Convert tree entries to workspace state
                    for entry in entries {
                        let path = std::path::PathBuf::from(&entry.name);
                        workspace_state.files.insert(path.clone(), entry.hash);
                        
                        // Create basic metadata
                        let mut metadata = fs::metadata(".")?; // Use current dir as template
                        let mut perms = metadata.permissions();
                        perms.set_mode(entry.mode);
                        metadata = fs::metadata(".")?; // Re-read after permission change
                        workspace_state.metadata.insert(path, metadata);
                    }
                    
                    // Check for conflicts
                    let conflicts = workspace_state.check_conflicts(&repo)?;
                    if !conflicts.is_empty() {
                        println!("Conflicts detected when applying stash:");
                        for conflict in &conflicts {
                            println!("  {}", conflict.display());
                        }
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Cannot apply stash due to conflicts",
                        ));
                    }
                    
                    // Apply the workspace state
                    workspace_state.apply_to_workspace(&repo)?;
                    println!("Applied stash '{}'", stash_ref);
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Stash tree is not a valid tree object",
                    ));
                }
            }
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Stash does not point to a commit",
            ));
        }
    }
    
    Ok(())
}

pub fn drop(stash_ref: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
    
    ref_store.drop_stash(stash_ref)?;
    println!("Dropped stash '{}'", stash_ref);
    
    Ok(())
}

/// Shows a diff between two trees
fn show_diff(repo: &Repository, parent_hash: &str, stash_tree: &str) -> io::Result<()> {
    // Read parent tree
    let parent_commit = crate::cobra::core::object::Object::read_from_objects_dir(&repo.git_dir, parent_hash)?;
    let parent_tree = match parent_commit {
        crate::cobra::core::object::Object::Commit { tree, .. } => tree,
        _ => return Ok(()),
    };
    
    // Read both trees
    let parent_tree_obj = crate::cobra::core::object::Object::read_from_objects_dir(&repo.git_dir, &parent_tree)?;
    let stash_tree_obj = crate::cobra::core::object::Object::read_from_objects_dir(&repo.git_dir, stash_tree)?;
    
    match (parent_tree_obj, stash_tree_obj) {
        (crate::cobra::core::object::Object::Tree(parent_entries), crate::cobra::core::object::Object::Tree(stash_entries)) => {
            // Create maps for easy lookup
            let mut parent_map = std::collections::HashMap::new();
            for entry in parent_entries {
                parent_map.insert(entry.name.clone(), entry.hash);
            }
            
            let mut stash_map = std::collections::HashMap::new();
            for entry in stash_entries {
                stash_map.insert(entry.name.clone(), entry.hash);
            }
            
            // Show differences
            for (name, stash_hash) in &stash_map {
                if let Some(parent_hash) = parent_map.get(name) {
                    if parent_hash != stash_hash {
                        println!("diff --git a/{} b/{}", name, name);
                        println!("index {}..{}", &parent_hash[..7], &stash_hash[..7]);
                        println!("--- a/{}", name);
                        println!("+++ b/{}", name);
                        println!();
                    }
                } else {
                    println!("diff --git a/{} b/{}", name, name);
                    println!("new file mode 100644");
                    println!("index 0000000..{}", &stash_hash[..7]);
                    println!("--- /dev/null");
                    println!("+++ b/{}", name);
                    println!();
                }
            }
            
            // Show deleted files
            for (name, _) in &parent_map {
                if !stash_map.contains_key(name) {
                    println!("diff --git a/{} b/{}", name, name);
                    println!("deleted file mode 100644");
                    println!("index {}..0000000", &parent_map[name][..7]);
                    println!("--- a/{}", name);
                    println!("+++ /dev/null");
                    println!();
                }
            }
        }
        _ => {
            println!("diff --git a/ b/");
            println!("index {}..{}", &parent_tree[..7], &stash_tree[..7]);
            println!("--- a/");
            println!("+++ b/");
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_stash_push() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Set up a commit
        ref_store.update_ref("refs/heads/main", "main_commit")?;
        
        // Test stash push
        let stash_hash = ref_store.create_stash(Some("Test stash"))?;
        assert!(!stash_hash.is_empty());
        
        // Verify stash was created
        let stashes = ref_store.list_stashes()?;
        assert_eq!(stashes.len(), 1);
        
        Ok(())
    }

    #[test]
    fn test_stash_list() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Set up a commit
        ref_store.update_ref("refs/heads/main", "main_commit")?;
        
        // Create stashes
        ref_store.create_stash(Some("First stash"))?;
        ref_store.create_stash(Some("Second stash"))?;
        
        // Test list functionality
        let stashes = ref_store.list_stashes()?;
        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[0].0, "stash@{0}");
        assert_eq!(stashes[1].0, "stash@{1}");
        
        Ok(())
    }

    #[test]
    fn test_stash_show() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Set up a commit
        ref_store.update_ref("refs/heads/main", "main_commit")?;
        
        // Create a stash
        let stash_hash = ref_store.create_stash(Some("Test stash message"))?;
        
        // Test show functionality
        let retrieved_hash = ref_store.get_stash("stash@{0}")?;
        assert_eq!(retrieved_hash, Some(stash_hash));
        
        Ok(())
    }

    #[test]
    fn test_stash_drop() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Set up a commit
        ref_store.update_ref("refs/heads/main", "main_commit")?;
        
        // Create stashes
        ref_store.create_stash(Some("First stash"))?;
        ref_store.create_stash(Some("Second stash"))?;
        
        // Verify we have 2 stashes
        let stashes = ref_store.list_stashes()?;
        assert_eq!(stashes.len(), 2);
        
        // Drop first stash
        ref_store.drop_stash("stash@{0}")?;
        
        // Verify we have 1 stash left
        let stashes_after = ref_store.list_stashes()?;
        assert_eq!(stashes_after.len(), 1);
        
        Ok(())
    }

    #[test]
    fn test_stash_drop_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path().to_str().unwrap()).unwrap();
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
        
        // Try to drop non-existent stash
        let result = ref_store.drop_stash("stash@{0}");
        assert!(result.is_err());
        
        match result {
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::NotFound);
                assert!(e.to_string().contains("does not exist"));
            }
            _ => panic!("Expected error"),
        }
    }
} 