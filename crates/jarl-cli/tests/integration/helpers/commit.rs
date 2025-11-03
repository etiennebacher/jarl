use gix::Repository;
use gix::bstr::ByteSlice;
use std::path::{Path, PathBuf};

pub fn create_commit(file_path: PathBuf, repo: Repository) -> anyhow::Result<()> {
    let file_path = PathBuf::from(Path::file_name(&file_path).unwrap());

    // Get the work directory
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("No workdir"))?;

    // 1. Read the file and write it as a blob
    let full_path = workdir.join(&file_path);
    let content = std::fs::read(&full_path)?;
    let blob_id = repo.write_blob(&content)?;

    // 2. Create a tree with this blob
    let tree = gix::objs::Tree {
        entries: vec![gix::objs::tree::Entry {
            mode: gix::objs::tree::EntryKind::Blob.into(),
            filename: file_path.as_os_str().to_string_lossy().as_bytes().into(),
            oid: blob_id.detach(),
        }]
        .into(),
    };
    let tree_id = repo.write_object(&tree)?;

    // 3. Create author/committer signature
    let author = gix::actor::Signature {
        name: "Your Name".into(),
        email: "your@example.com".into(),
        time: gix::date::Time::now_local_or_utc(),
    };

    // 4. Create the commit (no parents means initial commit)
    let commit = gix::objs::Commit {
        message: "Initial commit".into(),
        tree: tree_id.detach(),
        author: author.clone(),
        committer: author,
        encoding: None,
        parents: vec![].into(),
        extra_headers: vec![].into(),
    };
    let commit_id = repo.write_object(&commit)?;

    // 5. Update HEAD to point to this commit
    // In a fresh repo, HEAD exists but points to a non-existent branch (refs/heads/main or master)
    // We need to create that reference
    let head = repo.head()?;
    let head_ref_name = if let Some(name) = head.referent_name() {
        name.as_bstr().to_owned()
    } else {
        "refs/heads/main".into()
    };

    repo.reference(
        gix::refs::FullName::try_from(head_ref_name.as_bstr())?,
        commit_id.detach(),
        gix::refs::transaction::PreviousValue::Any,
        "commit (initial): Initial commit",
    )?;

    // 6. Create an index from the tree so the file is marked as tracked
    let tree_obj = repo.find_object(tree_id.detach())?;
    let tree_for_index = tree_obj.try_into_tree()?;
    let mut index = repo.index_from_tree(&tree_for_index.id)?;

    // Write the index to disk
    index.write(gix::index::write::Options::default())?;

    Ok(())
}
