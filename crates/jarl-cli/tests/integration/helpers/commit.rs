use git2::{Repository, Signature};
use std::path::{Path, PathBuf};

pub fn create_commit(file_path: PathBuf, repo: Repository) -> anyhow::Result<()> {
    let file_path = PathBuf::from(Path::file_name(&file_path).unwrap());

    // 1. Add the file to the index
    let mut index = repo.index()?;
    index.add_path(&file_path)?;
    index.write()?; // Write the index to disk

    // 2. Write the index to a tree
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    // 3. Create a signature
    let sig = Signature::now("Your Name", "your@example.com")?;

    // 4. Commit (no parents means it's the initial commit)
    let _ = repo.commit(
        Some("HEAD"),     // Point HEAD to this commit
        &sig,             // Author
        &sig,             // Committer
        "Initial commit", // Commit message
        &tree,            // Tree
        &[],              // Parents
    )?;

    Ok(())
}
