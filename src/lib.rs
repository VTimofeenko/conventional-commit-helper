pub mod test_utils {
    use git2::{Oid, Repository, Signature};
    use itertools::Itertools;
    use log::debug;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Set up a fake repo with commits based on the argument
    /// tmpdir is passed as a param so that it's created in the calling test
    pub fn setup_repo_with_commits_and_files(
        tmpdir: &Path,
        commit_msgs: &[&str],
        files: &[&str],
    ) -> Repository {
        let repo = Repository::init(tmpdir).unwrap();
        debug!(
            "Setting up a repo at {:?} with {:?} commits (including initial)",
            tmpdir,
            commit_msgs.len()
        );

        debug!("{}", commit_msgs.iter().zip_longest(files.iter()).len());

        let mut parent_commit: Option<Oid> = None;

        // Iterate over commit messages and files (if those exist) commit messages are the ones
        // that are more important for tests so if there is no file specified -- generated fake
        // commit will just touch a fallback file
        //
        // The fake commits will write the commit message into the file.
        commit_msgs.iter().zip_longest(files).for_each(|pair| {
            let commit_msg = pair.clone().left().unwrap();
            let file = pair.right().unwrap_or(&"default_file");
            debug!("Setting up commit with message '{:?}'", commit_msg);
            let file_path = tmpdir.join(file);
            debug!("It should go into the file {:?}", file_path);
            debug!("Writing garbage to {:?}", file_path);
            fs::write(file_path, commit_msg).unwrap();

            let mut index = repo.index().unwrap();
            let _ = index.add_path(Path::new(file)); // File has to be relative to the repo to be
                                                     // committed
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

    pub fn setup_repo_with_commits(tmpdir: &Path, commit_msgs: &[&str]) -> git2::Repository {
        setup_repo_with_commits_and_files(tmpdir, commit_msgs, &[])
    }

    const TYPES_ONLY_CONFIG: &str = r#"
                [types]
                foo = "bar"
                "#;

    pub fn mk_config_with_types_only(tmpdir: &Path) {
        setup_config_file_in_path(tmpdir, TYPES_ONLY_CONFIG);
    }

    const SCOPES_ONLY_CONFIG: &str = r#"
                [scopes]
                foz = "baz"
                "#;

    pub fn mk_config_with_scopes_only(tmpdir: &Path) {
        setup_config_file_in_path(tmpdir, SCOPES_ONLY_CONFIG);
    }

    pub fn mk_config_full(tmpdir: &Path) {
        setup_config_file_in_path(tmpdir, &(TYPES_ONLY_CONFIG.to_owned() + SCOPES_ONLY_CONFIG));
    }

    pub fn setup_config_file_in_path(tmpdir: &Path, content: &str) -> PathBuf {
        debug!("Setting up config file at {:?}", tmpdir);
        let config_path = tmpdir.join(".dev/conventional-commit-helper.json");
        let _ = fs::create_dir_all(config_path.parent().unwrap());
        fs::write(&config_path, content).unwrap();

        config_path
    }
}
