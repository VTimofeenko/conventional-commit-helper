pub mod test_utils {
    use std::path::Path;
    use std::fs;
    use git2::{Repository, Oid, Signature};

    /// Set up a fake repo with commits based on the argument
    /// tmpdir is passed as a param so that it's created in the calling test
    pub fn setup_repo_with_commits(tmpdir: &Path, commit_msgs: &[&str]) -> Repository {
        let repo = Repository::init(tmpdir).unwrap();

        let mut parent_commit: Option<Oid> = None;

        commit_msgs.iter().for_each(|commit_msg| {
            let file_path = tmpdir.join("helloworld");
            fs::write(file_path, commit_msg).unwrap();

            let mut index = repo.index().unwrap();
            let _ = index.add_path(Path::new("helloworld"));
            let _ = index.write();

            let sig = Signature::now("nobody", "nobody@example.com").unwrap();

            let tree_id = index.write_tree().unwrap();

            let tree = repo.find_tree(tree_id).unwrap();

            let parents = match parent_commit {
                Some(parent_id) => vec![repo.find_commit(parent_id).unwrap()],
                None => vec![], // No parent for the first commit
            };
            let commit_id = repo
                .commit(
                    Some("HEAD"),                        // Update HEAD
                    &sig,                                // Author
                    &sig,                                // Committer
                    commit_msg,                          // Commit message
                    &tree,                               // Tree
                    &parents.iter().collect::<Vec<_>>(), // Parent commits
                )
                .unwrap();

            // Update the parent_commit for the next iteration
            parent_commit = Some(commit_id);
        });

        repo
    }
}
