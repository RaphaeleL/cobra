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

    pub fn list_branches(&self) -> io::Result<Vec<String>> {
        let heads_dir = self.git_dir.join("refs/heads");
        if !heads_dir.exists() {
            return Ok(Vec::new());
        }

        let mut branches = Vec::new();
        for entry in fs::read_dir(heads_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    branches.push(name.to_string());
                }
            }
        }
        Ok(branches)
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
        assert!(branches.contains(&"main".to_string()));
        assert!(branches.contains(&"feature1".to_string()));
        assert!(branches.contains(&"feature2".to_string()));
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
} 