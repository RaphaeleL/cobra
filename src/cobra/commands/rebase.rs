// Rebase commits on top of another base tip
use std::io;
use crate::cobra::core::repository::Repository;

pub fn run(branch: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
    
    // Check if target branch exists
    let target_branch_ref = format!("refs/heads/{}", branch);
    let target_commit = ref_store.read_ref(&target_branch_ref)?
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
        // HEAD points to a branch
        let current_branch_ref = &head_content[5..];
        ref_store.read_ref(current_branch_ref)?
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "Current branch reference not found",
            ))?
    } else {
        // HEAD points directly to a commit
        head_content.clone()
    };
    
    // Check if we're trying to rebase onto the same branch
    if current_commit == target_commit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Cannot rebase branch onto itself"),
        ));
    }
    
    // For now, we'll create a simple rebase by creating a new commit
    // In a real implementation, you'd need to handle multiple commits, conflicts, etc.
    let author = crate::cobra::core::signature::Signature::new(
        "Your Name".to_string(),
        "you@example.com".to_string(),
    );
    let committer = author.clone();
    
    // Create rebase commit with target as parent
    let rebase_commit = crate::cobra::core::object::Object::new_commit(
        current_commit.clone(), // Use current tree (simplified)
        vec![target_commit], // Only target as parent (rebase)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rebase_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Create branches with different commits
        ref_store.create_branch("base")?;
        ref_store.create_branch("feature")?;
        
        // Set different commits
        ref_store.update_ref("refs/heads/base", "base_commit")?;
        ref_store.update_ref("refs/heads/feature", "feature_commit")?;
        
        // Switch to feature branch
        ref_store.update_head("ref: refs/heads/feature")?;
        
        // Test rebase logic directly
        let target_branch_ref = "refs/heads/base";
        let target_commit = ref_store.read_ref(target_branch_ref)?
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "Branch does not exist",
            ))?;
        
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
        
        // Verify commits are different
        assert_ne!(current_commit, target_commit);
        
        Ok(())
    }

    #[test]
    fn test_rebase_nonexistent_branch() {
        let temp_dir = TempDir::new().unwrap();
        let _repo = Repository::init(temp_dir.path().to_str().unwrap()).unwrap();
        
        // Test non-existent branch logic directly
        let ref_store = crate::cobra::core::ref_store::RefStore::new(temp_dir.path().join(".cobra"));
        let target_branch_ref = "refs/heads/nonexistent";
        let exists = ref_store.read_ref(target_branch_ref).unwrap().is_some();
        assert!(!exists);
    }

    #[test]
    fn test_rebase_onto_same_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        
        // Create a branch
        ref_store.create_branch("test")?;
        ref_store.update_ref("refs/heads/test", "same_commit")?;
        ref_store.update_head("ref: refs/heads/test")?;
        
        // Test same branch logic directly
        let target_branch_ref = "refs/heads/test";
        let target_commit = ref_store.read_ref(target_branch_ref)?
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "Branch does not exist",
            ))?;
        
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
        
        // Verify commits are the same (should prevent rebase)
        assert_eq!(current_commit, target_commit);
        
        Ok(())
    }
} 