// Reference management (branches, tags, HEAD)
use std::fs;
use std::io;
use std::path::PathBuf;

pub struct RefStore {
    git_dir: PathBuf,
}

impl RefStore {
    pub fn new(git_dir: PathBuf) -> Self {
        RefStore { git_dir }
    }

    pub fn create_initial_refs(&self) -> io::Result<()> {
        // Create refs directory structure
        let refs_dir = self.git_dir.join("refs");
        let heads_dir = refs_dir.join("heads");
        fs::create_dir_all(&heads_dir)?;

        // Create empty main branch reference
        let main_ref = heads_dir.join("main");
        fs::write(&main_ref, "")?;

        // Create HEAD pointing to main branch
        let head_path = self.git_dir.join("HEAD");
        fs::write(head_path, "ref: refs/heads/main\n")?;

        Ok(())
    }

    pub fn update_ref(&self, ref_name: &str, target: &str) -> io::Result<()> {
        let ref_path = self.git_dir.join(ref_name);
        
        // Create parent directories if they don't exist
        if let Some(parent) = ref_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(ref_path, format!("{}\n", target))
    }

    pub fn read_ref(&self, ref_name: &str) -> io::Result<Option<String>> {
        let ref_path = self.git_dir.join(ref_name);
        
        if !ref_path.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(ref_path)?;
        Ok(Some(content.trim().to_string()))
    }

    pub fn read_head(&self) -> io::Result<Option<String>> {
        self.read_ref("HEAD")
    }

    pub fn update_head(&self, target: &str) -> io::Result<()> {
        self.update_ref("HEAD", target)
    }

    pub fn create_branch(&self, branch_name: &str) -> io::Result<()> {
        // Check if branch already exists
        let branch_ref = format!("refs/heads/{}", branch_name);
        if let Some(_) = self.read_ref(&branch_ref)? {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("A branch named '{}' already exists", branch_name),
            ));
        }

        // Get current HEAD commit
        let head_content = self.read_head()?;
        let current_commit = match head_content {
            Some(content) if content.starts_with("ref: ") => {
                // HEAD points to a branch, get the commit from that branch
                let branch_name = content.strip_prefix("ref: ").unwrap().trim();
                self.read_ref(branch_name)?
                    .ok_or_else(|| io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Branch '{}' not found", branch_name),
                    ))?
            },
            Some(commit_hash) if !commit_hash.is_empty() => {
                // HEAD points directly to a commit
                commit_hash
            },
            _ => {
                // No commits yet, create empty branch
                "".to_string()
            }
        };

        // Create the new branch pointing to the current commit
        self.update_ref(&branch_ref, &current_commit)
    }

    pub fn list_branches(&self) -> io::Result<Vec<(String, String)>> {
        let heads_dir = self.git_dir.join("refs/heads");
        if !heads_dir.exists() {
            return Ok(Vec::new());
        }

        let mut branches = Vec::new();
        for entry in fs::read_dir(heads_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    let branch_ref = format!("refs/heads/{}", name);
                    if let Some(hash) = self.read_ref(&branch_ref)? {
                        branches.push((name.to_string(), hash));
                    }
                }
            }
        }
        Ok(branches)
    }

    pub fn delete_branch(&self, branch_name: &str) -> io::Result<()> {
        // Check if branch exists
        let branch_ref = format!("refs/heads/{}", branch_name);
        if self.read_ref(&branch_ref)?.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Branch '{}' does not exist", branch_name),
            ));
        }

        // Check if we're trying to delete the current branch
        let head_content = self.read_head()?;
        if let Some(content) = head_content {
            if content == format!("ref: {}", branch_ref) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Cannot delete the current branch '{}'", branch_name),
                ));
            }
        }

        // Delete the branch file
        let branch_path = self.git_dir.join(&branch_ref);
        fs::remove_file(branch_path)?;
        
        Ok(())
    }

    pub fn merge_branch(&self, branch_name: &str) -> io::Result<()> {
        // Check if branch exists
        let branch_ref = format!("refs/heads/{}", branch_name);
        let branch_commit = self.read_ref(&branch_ref)?
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                format!("Branch '{}' does not exist", branch_name),
            ))?;

        // Get current branch commit
        let head_content = self.read_head()?
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "HEAD reference not found",
            ))?;

        let current_commit = if head_content.starts_with("ref: ") {
            // HEAD points to a branch, get the commit from that branch
            let current_branch_ref = &head_content[5..];
            self.read_ref(current_branch_ref)?
                .ok_or_else(|| io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Branch '{}' not found", current_branch_ref),
                ))?
        } else {
            // HEAD points directly to a commit
            head_content.clone()
        };

        // Check if we're trying to merge the same branch
        if current_commit == branch_commit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Cannot merge branch '{}' into itself", branch_name),
            ));
        }

        // For now, we'll create a simple merge commit
        // In a real implementation, you'd need to handle conflicts, etc.
        let author = crate::cobra::core::signature::Signature::new(
            "Your Name".to_string(),
            "you@example.com".to_string(),
        );
        let committer = author.clone();

        // Create merge commit with both parents
        let merge_commit = crate::cobra::core::object::Object::new_commit(
            current_commit.clone(), // Use current tree (simplified)
            vec![current_commit, branch_commit],
            author,
            committer,
            format!("Merge branch '{}'", branch_name),
        );

        // Write merge commit
        let merge_hash = merge_commit.hash();
        merge_commit.write_to_objects_dir(&self.git_dir)?;

        // Update current branch to point to merge commit
        if head_content.starts_with("ref: ") {
            let current_branch_ref = &head_content[5..];
            self.update_ref(current_branch_ref, &merge_hash)?;
        } else {
            self.update_head(&merge_hash)?;
        }

        Ok(())
    }

    pub fn create_stash(&self, message: Option<&str>) -> io::Result<String> {
        // Create repository instance
        let repo = crate::cobra::core::repository::Repository::open(".")?;
        
        // Create stash state from current workspace and index
        let stash_message = message.unwrap_or("WIP on current branch");
        let stash_state = crate::cobra::core::workspace::StashState::create(&repo, stash_message)?;
        
        // Create commit from stash state
        let stash_hash = stash_state.create_commit(&repo)?;
        
        // Add to stash list
        self.add_to_stash_list(&stash_hash)?;
        
        Ok(stash_hash)
    }

    pub fn list_stashes(&self) -> io::Result<Vec<(String, String)>> {
        let stash_list_path = self.git_dir.join("refs/stash");
        if !stash_list_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&stash_list_path)?;
        let mut stashes = Vec::new();
        
        for (index, line) in content.lines().enumerate() {
            if !line.trim().is_empty() {
                stashes.push((format!("stash@{{{}}}", index), line.trim().to_string()));
            }
        }

        Ok(stashes)
    }

    pub fn get_stash(&self, stash_ref: &str) -> io::Result<Option<String>> {
        let stashes = self.list_stashes()?;
        
        // Parse stash reference like "stash@{0}"
        if stash_ref.starts_with("stash@{") && stash_ref.ends_with("}") {
            let index_str = &stash_ref[7..stash_ref.len()-1];
            if let Ok(index) = index_str.parse::<usize>() {
                if index < stashes.len() {
                    return Ok(Some(stashes[index].1.clone()));
                }
            }
        }
        
        // Try direct hash
        if stash_ref.len() == 40 {
            return Ok(Some(stash_ref.to_string()));
        }

        Ok(None)
    }

    pub fn drop_stash(&self, stash_ref: &str) -> io::Result<()> {
        let stashes = self.list_stashes()?;
        let stash_list_path = self.git_dir.join("refs/stash");
        
        // Parse stash reference
        let index = if stash_ref.starts_with("stash@{") && stash_ref.ends_with("}") {
            let index_str = &stash_ref[7..stash_ref.len()-1];
            index_str.parse::<usize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidInput, "Invalid stash reference")
            })?
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid stash reference format",
            ));
        };

        if index >= stashes.len() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Stash '{}' does not exist", stash_ref),
            ));
        }

        // Remove the stash from the list
        let mut new_stashes = stashes;
        new_stashes.remove(index);
        
        // Write updated stash list
        let content = new_stashes.iter()
            .map(|(_, hash)| hash.clone())
            .collect::<Vec<_>>()
            .join("\n");
        
        if content.is_empty() {
            fs::remove_file(&stash_list_path)?;
        } else {
            fs::write(&stash_list_path, content)?;
        }

        Ok(())
    }

    fn add_to_stash_list(&self, stash_hash: &str) -> io::Result<()> {
        let stash_list_path = self.git_dir.join("refs/stash");
        
        // Create refs directory if it doesn't exist
        if let Some(parent) = stash_list_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Append to stash list
        let mut content = if stash_list_path.exists() {
            fs::read_to_string(&stash_list_path)?
        } else {
            String::new()
        };

        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(stash_hash);

        fs::write(&stash_list_path, content)?;
        Ok(())
    }

    pub fn switch_branch(&self, branch_name: &str) -> io::Result<()> {
        let branch_ref = format!("refs/heads/{}", branch_name);
        if self.read_ref(&branch_ref)?.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound, 
                format!("Branch '{}' does not exist", branch_name)
            ));
        }
        self.update_head(&format!("ref: {}", branch_ref))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
        // Create a new branch
        ref_store.create_branch("feature")?;
        
        // Verify branch was created
        let branch_content = ref_store.read_ref("refs/heads/feature")?;
        assert!(branch_content.is_some());
        
        // Verify it points to the same commit as main (empty in this case)
        let main_content = ref_store.read_ref("refs/heads/main")?;
        assert_eq!(branch_content, main_content);
        
        Ok(())
    }

    #[test]
    fn test_create_branch_with_commit() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
        // Set main branch to point to a commit
        let commit_hash = "abc123def456";
        ref_store.update_ref("refs/heads/main", commit_hash)?;
        
        // Create a new branch
        ref_store.create_branch("feature")?;
        
        // Verify branch points to the same commit
        let branch_content = ref_store.read_ref("refs/heads/feature")?;
        assert_eq!(branch_content, Some(commit_hash.to_string()));
        
        Ok(())
    }

    #[test]
    fn test_create_duplicate_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
        // Create a branch
        ref_store.create_branch("feature")?;
        
        // Try to create the same branch again
        let result = ref_store.create_branch("feature");
        assert!(result.is_err());
        
        match result {
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::AlreadyExists);
                assert!(e.to_string().contains("already exists"));
            }
            _ => panic!("Expected error"),
        }
        
        Ok(())
    }

    #[test]
    fn test_list_branches() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
        // Create some branches
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
    fn test_list_branches_empty() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Don't initialize refs, so no branches exist
        let branches = ref_store.list_branches()?;
        assert_eq!(branches.len(), 0);
        
        Ok(())
    }

    #[test]
    fn test_delete_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
        // Create a branch
        ref_store.create_branch("feature")?;
        
        // Verify branch exists
        let branches = ref_store.list_branches()?;
        let branch_names: Vec<String> = branches.iter().map(|(name, _)| name.clone()).collect();
        assert!(branch_names.contains(&"feature".to_string()));
        
        // Delete the branch
        ref_store.delete_branch("feature")?;
        
        // Verify branch is gone
        let branches_after = ref_store.list_branches()?;
        let branch_names_after: Vec<String> = branches_after.iter().map(|(name, _)| name.clone()).collect();
        assert!(!branch_names_after.contains(&"feature".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_delete_nonexistent_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
        // Try to delete a non-existent branch
        let result = ref_store.delete_branch("nonexistent");
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
    fn test_delete_current_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
        // Create a branch
        ref_store.create_branch("feature")?;
        
        // Switch to the branch
        ref_store.update_head("ref: refs/heads/feature")?;
        
        // Try to delete the current branch
        let result = ref_store.delete_branch("feature");
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
    fn test_merge_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
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
    fn test_merge_nonexistent_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
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

    #[test]
    fn test_merge_same_branch() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
        // Set same commit for both branches
        ref_store.update_ref("refs/heads/main", "same_commit")?;
        ref_store.update_ref("refs/heads/feature", "same_commit")?;
        
        // Try to merge the same branch
        let result = ref_store.merge_branch("feature");
        assert!(result.is_err());
        
        match result {
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
                assert!(e.to_string().contains("Cannot merge branch"));
            }
            _ => panic!("Expected error"),
        }
        
        Ok(())
    }

    #[test]
    fn test_create_stash() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs and create repository structure
        ref_store.create_initial_refs()?;
        ref_store.update_ref("refs/heads/main", "main_commit")?;
        
        // Create objects directory for stash creation
        fs::create_dir_all(temp_dir.path().join(".cobra/objects"))?;
        
        // Create a stash
        let stash_hash = ref_store.create_stash(Some("Test stash"))?;
        assert!(!stash_hash.is_empty());
        
        // Verify stash was added to list
        let stashes = ref_store.list_stashes()?;
        assert_eq!(stashes.len(), 1);
        assert_eq!(stashes[0].0, "stash@{0}");
        assert_eq!(stashes[0].1, stash_hash);
        
        Ok(())
    }

    #[test]
    fn test_list_stashes() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs and create repository structure
        ref_store.create_initial_refs()?;
        ref_store.update_ref("refs/heads/main", "main_commit")?;
        
        // Create objects directory for stash creation
        fs::create_dir_all(temp_dir.path().join(".cobra/objects"))?;
        
        // Create multiple stashes
        ref_store.create_stash(Some("First stash"))?;
        ref_store.create_stash(Some("Second stash"))?;
        
        // List stashes
        let stashes = ref_store.list_stashes()?;
        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[0].0, "stash@{0}");
        assert_eq!(stashes[1].0, "stash@{1}");
        
        Ok(())
    }

    #[test]
    fn test_get_stash() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs and create repository structure
        ref_store.create_initial_refs()?;
        ref_store.update_ref("refs/heads/main", "main_commit")?;
        
        // Create objects directory for stash creation
        fs::create_dir_all(temp_dir.path().join(".cobra/objects"))?;
        
        // Create a stash
        let stash_hash = ref_store.create_stash(Some("Test stash"))?;
        
        // Get stash by reference
        let retrieved_hash = ref_store.get_stash("stash@{0}")?;
        assert_eq!(retrieved_hash, Some(stash_hash));
        
        // Get non-existent stash
        let non_existent = ref_store.get_stash("stash@{1}")?;
        assert_eq!(non_existent, None);
        
        Ok(())
    }

    #[test]
    fn test_drop_stash() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs and create repository structure
        ref_store.create_initial_refs()?;
        ref_store.update_ref("refs/heads/main", "main_commit")?;
        
        // Create objects directory for stash creation
        fs::create_dir_all(temp_dir.path().join(".cobra/objects"))?;
        
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
        assert_eq!(stashes_after[0].0, "stash@{0}"); // Index should be updated
        
        Ok(())
    }

    #[test]
    fn test_drop_nonexistent_stash() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let ref_store = RefStore::new(temp_dir.path().to_path_buf());
        
        // Initialize refs
        ref_store.create_initial_refs()?;
        
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
        
        Ok(())
    }
} 