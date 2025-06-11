// Repository management 

use std::fs;
use std::path::{Path, PathBuf};
use std::io;
use crate::cobra::core::ref_store::RefStore;
use crate::cobra::core::index::Index;

pub struct Repository {
    pub root_path: PathBuf,
    pub git_dir: PathBuf,
    pub index: Index,
}

impl Repository {
    pub fn init(path: &str) -> io::Result<Repository> {
        let root_path = PathBuf::from(path);
        let git_dir = root_path.join(".cobra");
        
        // Create .cobra directory and its subdirectories
        fs::create_dir_all(&git_dir)?;
        fs::create_dir_all(git_dir.join("objects"))?;
        fs::create_dir_all(git_dir.join("refs/heads"))?;

        // Create HEAD file pointing to refs/heads/main
        fs::write(
            git_dir.join("HEAD"),
            "ref: refs/heads/main\n",
        )?;

        let repo = Repository {
            root_path,
            git_dir,
            index: Index::new(),
        };

        // Initialize refs
        let ref_store = RefStore::new(repo.git_dir.clone());
        ref_store.create_initial_refs()?;
        
        // Save empty index
        repo.save_index()?;
        
        Ok(repo)
    }

    /// Checks if a repository exists at the given path
    #[allow(dead_code)]
    pub fn exists(path: &str) -> bool {
        let cobra_dir = Path::new(path).join(".cobra");
        cobra_dir.exists() && cobra_dir.is_dir()
    }

    pub fn open(path: &str) -> io::Result<Repository> {
        let root_path = PathBuf::from(path);
        let git_dir = root_path.join(".cobra");

        if !git_dir.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Not a cobra repository (or any of the parent directories)",
            ));
        }

        // Try to load existing index
        let index = Index::load(&Repository {
            root_path: root_path.clone(),
            git_dir: git_dir.clone(),
            index: Index::new(),
        })?;

        Ok(Repository {
            root_path,
            git_dir,
            index,
        })
    }

    pub fn add_to_index(&mut self, entry: crate::cobra::core::index::IndexEntry) -> io::Result<()> {
        self.index.add_entry(entry);
        self.save_index()
    }

    fn save_index(&self) -> io::Result<()> {
        let index_path = Path::new(&self.git_dir).join("index");
        self.index.write_to_file(&index_path)
    }
} 