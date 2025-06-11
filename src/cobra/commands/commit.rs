use std::io;
use crate::cobra::core::{
    repository::Repository,
    object::Object,
    ref_store::RefStore,
    tree::build_tree_from_index,
    signature::Signature,
};

pub fn run(message: &str) -> io::Result<()> {
    // Open repository
    let repo = Repository::open(".")?;
    let ref_store = RefStore::new(repo.git_dir.clone());

    // Build tree from index
    let tree = build_tree_from_index(&repo)?;
    let tree_hash = tree.hash();
    tree.write_to_objects_dir(&repo.git_dir)?;

    // Get parent commit hash from HEAD
    let parent_hash = ref_store.read_head()?
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

    // Create author and committer signatures
    let author = Signature::new("Your Name".to_string(), "you@example.com".to_string());
    let committer = author.clone();

    // Create commit object
    let commit = Object::new_commit(
        tree_hash.clone(),
        if parent_hash.is_empty() { vec![] } else { vec![parent_hash.clone()] },
        author,
        committer,
        message.to_string(),
    );

    // Write commit object
    let commit_hash = commit.hash();
    commit.write_to_objects_dir(&repo.git_dir)?;

    // Update HEAD
    let head_ref = ref_store.read_head()?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HEAD reference not found"))?;

    if head_ref.starts_with("ref: ") {
        // HEAD points to a branch, update the branch
        let branch_ref = &head_ref[5..];
        ref_store.update_ref(branch_ref, &commit_hash)?;
    } else {
        // HEAD points directly to a commit, update HEAD
        ref_store.update_head(&commit_hash)?;
    }

    println!("[{}] {}", &commit_hash[..7], message);

    Ok(())
} 