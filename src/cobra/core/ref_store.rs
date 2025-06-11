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
} 