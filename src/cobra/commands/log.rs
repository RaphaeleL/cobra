use std::io;
use crate::cobra::core::{
    repository::Repository,
    object::Object,
    ref_store::RefStore,
};

pub fn run() -> io::Result<()> {
    let repo = Repository::open(".")?;
    let ref_store = RefStore::new(repo.git_dir.clone());

    // Get current commit hash from HEAD
    let mut current_hash = ref_store.read_head()?
        .and_then(|head_ref| {
            if head_ref.starts_with("ref: ") {
                // HEAD points to a branch
                let branch_ref = &head_ref[5..];
                ref_store.read_ref(branch_ref).ok().flatten()
            } else {
                // HEAD points directly to a commit
                Some(head_ref)
            }
        })
        .unwrap_or_default();

    // Print commit history
    while !current_hash.is_empty() {
        let commit = Object::read_from_objects_dir(&repo.git_dir, &current_hash)?;
        match commit {
            Object::Commit { tree: _, parents, author, committer: _, message } => {
                println!("commit {}", current_hash);
                println!("Author: {} <{}>", author.name, author.email);
                println!("Date:   {} {}", author.timestamp, author.timezone);
                println!();
                for line in message.lines() {
                    println!("    {}", line);
                }
                println!();

                // Move to parent commit
                current_hash = parents.first().cloned().unwrap_or_default();
            }
            _ => break,
        }
    }

    Ok(())
} 