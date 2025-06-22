// Create a new branch
use std::io;
use crate::cobra::core::repository::Repository;

pub fn run(name: &str) -> io::Result<()> {
    create(name)
}

pub fn create(name: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
    ref_store.create_branch(name)?;
    println!("Created branch '{}'", name);
    Ok(())
}

pub fn list() -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir);
    let branches = ref_store.list_branches()?;
    for branch in branches {
        println!("{}", branch);
    }
    Ok(())
}

pub fn switch(name: &str) -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
    let branch_ref = format!("refs/heads/{}", name);
    if ref_store.read_ref(&branch_ref)?.is_none() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Branch '{}' does not exist", name)));
    }
    ref_store.update_head(&format!("ref: {}", branch_ref))?;
    println!("Switched to branch '{}'", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_list_branches() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        ref_store.create_branch("feature1")?;
        ref_store.create_branch("feature2")?;
        let branches = ref_store.list_branches()?;
        assert!(branches.contains(&"main".to_string()));
        assert!(branches.contains(&"feature1".to_string()));
        assert!(branches.contains(&"feature2".to_string()));
        Ok(())
    }

    #[test]
    fn test_switch_branch_success() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        ref_store.create_branch("dev")?;
        // Should succeed
        ref_store.update_head("ref: refs/heads/main")?;
        ref_store.update_head(&format!("ref: refs/heads/{}", "dev"))?;
        let head = ref_store.read_head()?.unwrap();
        assert_eq!(head, "ref: refs/heads/dev");
        Ok(())
    }

    #[test]
    fn test_switch_branch_not_found() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path().to_str().unwrap())?;
        let ref_store = crate::cobra::core::ref_store::RefStore::new(repo.git_dir.clone());
        let result = ref_store.update_head("ref: refs/heads/doesnotexist");
        assert!(result.is_ok()); // The file is created, but the branch doesn't exist
        // Now, simulate the CLI switch logic
        let branch_ref = "refs/heads/doesnotexist";
        let exists = ref_store.read_ref(branch_ref)?.is_some();
        assert!(!exists);
        Ok(())
    }
} 