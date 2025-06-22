// Branch management commands
use std::io;
use crate::cobra::core::repository::Repository;

pub fn list() -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
    
    let branches = ref_store.list_branches()?;
    
    if branches.is_empty() {
        println!("No branches found");
        return Ok(());
    }
    
    // Get current branch name
    let head_content = ref_store.read_head()?;
    let current_branch = if let Some(content) = head_content {
        if content.starts_with("ref: ") {
            let branch_ref = content.strip_prefix("ref: ").unwrap().trim();
            branch_ref.strip_prefix("refs/heads/").unwrap_or(branch_ref).to_string()
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    };
    
    for (name, hash) in branches {
        let current_marker = if name == current_branch { " *" } else { "" };
        println!("{}{} {}", name, current_marker, &hash[..7]);
    }
    
    Ok(())
}

pub fn create(name: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
    
    ref_store.create_branch(name)?;
    println!("Created branch '{}'", name);
    
    Ok(())
}

pub fn switch(name: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
    
    ref_store.switch_branch(name)?;
    println!("Switched to branch '{}'", name);
    
    Ok(())
}

pub fn delete(name: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
    
    ref_store.delete_branch(name)?;
    println!("Deleted branch '{}'", name);
    
    Ok(())
}

pub fn merge(name: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
    
    ref_store.merge_branch(name)?;
    println!("Merged branch '{}' into current branch", name);
    
    Ok(())
}

pub fn rebase(branch: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
    
    // Check if target branch exists
    let branch_ref = format!("refs/heads/{}", branch);
    let target_commit = ref_store.read_ref(&branch_ref)?
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::NotFound,
            format!("Branch '{}' does not exist", branch),
        ))?;

    // Get current branch commit
    let head_content = ref_store.read_head()?
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::NotFound,
            "HEAD reference not found",
        ))?;

    let current_commit = if head_content.starts_with("ref: ") {
        let current_branch_ref = &head_content[5..];
        ref_store.read_ref(current_branch_ref)?
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "Current branch reference not found",
            ))?
    } else {
        head_content.clone()
    };

    // Check if we're trying to rebase onto the same branch
    if current_commit == target_commit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Cannot rebase branch '{}' onto itself", branch),
        ));
    }

    // Create a new commit with the target branch as parent
    let author = crate::cobra::core::signature::Signature::new(
        "Your Name".to_string(),
        "you@example.com".to_string(),
    );
    let committer = author.clone();

    let rebase_commit = crate::cobra::core::object::Object::new_commit(
        current_commit.clone(), // Use current tree (simplified)
        vec![target_commit],
        author,
        committer,
        format!("Rebase onto {}", branch),
    );

    // Write rebase commit
    let rebase_hash = rebase_commit.hash();
    rebase_commit.write_to_objects_dir(&repo.git_dir)?;

    // Update current branch to point to rebase commit
    if head_content.starts_with("ref: ") {
        let current_branch_ref = &head_content[5..];
        ref_store.update_ref(current_branch_ref, &rebase_hash)?;
    } else {
        ref_store.update_head(&rebase_hash)?;
    }

    println!("Rebased current branch onto '{}'", branch);
    Ok(())
}

// Legacy function for backward compatibility
pub fn run(name: &str) -> io::Result<()> {
    create(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_list_branches() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
        
        // Create branches
        ref_store.create_branch("feature1")?;
        ref_store.create_branch("feature2")?;
        
        // List branches
        let branches = ref_store.list_branches()?;
        
        // Should contain main, feature1, and feature2
        let branch_names: Vec<String> = branches.iter().map(|(name, _)| name.clone()).collect();
        assert!(branch_names.contains(&"main".to_string()));
        assert!(branch_names.contains(&"feature1".to_string()));
        assert!(branch_names.contains(&"feature2".to_string()));
        assert_eq!(branches.len(), 3);
        
        Ok(())
    }

    #[test]
    fn test_switch_branch_success() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
        
        // Create a branch
        ref_store.create_branch("feature")?;
        
        // Switch to the branch
        ref_store.switch_branch("feature")?;
        
        // Verify HEAD points to the branch
        let head_content = ref_store.read_head()?;
        assert_eq!(head_content, Some("ref: refs/heads/feature".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_switch_branch_not_found() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
        
        // Try to switch to non-existent branch
        let result = ref_store.switch_branch("nonexistent");
        assert!(result.is_err());
        
        match result {
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::NotFound);
                assert!(e.to_string().contains("does not exist"));
            }
            _ => panic!("Expected error"),
        }
        
        Ok(())
    }

    #[test]
    fn test_delete_branch_command() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
        
        // Create a branch
        ref_store.create_branch("temp")?;
        
        // Verify branch exists
        let branches = ref_store.list_branches()?;
        let branch_names: Vec<String> = branches.iter().map(|(name, _)| name.clone()).collect();
        assert!(branch_names.contains(&"temp".to_string()));
        
        // Delete the branch
        ref_store.delete_branch("temp")?;
        
        // Verify branch is gone
        let branches_after = ref_store.list_branches()?;
        let branch_names_after: Vec<String> = branches_after.iter().map(|(name, _)| name.clone()).collect();
        assert!(!branch_names_after.contains(&"temp".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_delete_current_branch_command() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Create a branch and switch to it
        ref_store.create_branch("current")?;
        ref_store.update_head("ref: refs/heads/current")?;
        
        // Try to delete the current branch
        let result = ref_store.delete_branch("current");
        assert!(result.is_err());
        
        match result {
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
                assert!(e.to_string().contains("Cannot delete the current branch"));
            }
            _ => panic!("Expected error"),
        }
        
        Ok(())
    }

    #[test]
    fn test_merge_branch_command() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Create a branch
        ref_store.create_branch("feature")?;
        
        // Set some commits (simplified for testing)
        ref_store.update_ref("refs/heads/main", "main_commit")?;
        ref_store.update_ref("refs/heads/feature", "feature_commit")?;
        
        // Merge feature into main
        ref_store.merge_branch("feature")?;
        
        // Verify the merge created a new commit
        let main_commit = ref_store.read_ref("refs/heads/main")?;
        assert!(main_commit.is_some());
        assert_ne!(main_commit.unwrap(), "main_commit"); // Should be different after merge
        
        Ok(())
    }

    #[test]
    fn test_merge_nonexistent_branch_command() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Try to merge a non-existent branch
        let result = ref_store.merge_branch("nonexistent");
        assert!(result.is_err());
        
        match result {
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::NotFound);
                assert!(e.to_string().contains("does not exist"));
            }
            _ => panic!("Expected error"),
        }
        
        Ok(())
    }
} 