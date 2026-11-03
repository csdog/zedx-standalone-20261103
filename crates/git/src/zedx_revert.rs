use super::*;

impl RealGitRepository {
    pub(super) fn zedx_revert_with_mainline(
        &self,
        oid: Oid,
        mainline: Option<usize>,
        environment: Arc<HashMap<String, String>>,
    ) -> BoxFuture<'static, Result<()>> {
        let git = self.git_binary_in_worktree();
        self.executor
            .spawn(async move {
                let git = git?.envs((*environment).clone());
                anyhow::ensure!(
                    git.run(&["status", "--porcelain"]).await?.is_empty(),
                    "Commit or shelve working changes before reverting a commit"
                );
                let revision = oid.to_string();
                let parents = git
                    .run(&["rev-list", "--parents", "-n", "1", &revision])
                    .await?;
                let parent_count = parents.split_whitespace().count().saturating_sub(1);
                validate_mainline(parent_count, mainline)?;
                let mut arguments = vec!["revert".to_string(), "--no-edit".into()];
                if let Some(mainline) = mainline {
                    arguments.extend(["--mainline".into(), mainline.to_string()]);
                }
                arguments.push(revision);
                git.run(&arguments).await?;
                Ok(())
            })
            .boxed()
    }
}
fn validate_mainline(parent_count: usize, mainline: Option<usize>) -> Result<()> {
    match (parent_count, mainline) {
        (0 | 1, None) => Ok(()),
        (0 | 1, Some(_)) => bail!("A mainline parent can only be chosen for a merge commit"),
        (_, Some(parent)) if parent > 0 && parent <= parent_count => Ok(()),
        (_, Some(_)) => bail!("The selected mainline parent is outside this commit's parent list"),
        (_, None) => bail!("Choose a mainline parent before reverting a merge commit"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merge_requires_an_existing_one_based_parent() {
        assert!(validate_mainline(2, None).is_err());
        assert!(validate_mainline(2, Some(0)).is_err());
        assert!(validate_mainline(2, Some(3)).is_err());
        assert!(validate_mainline(2, Some(1)).is_ok());
        assert!(validate_mainline(2, Some(2)).is_ok());
        assert!(validate_mainline(4, Some(4)).is_ok());
        assert!(validate_mainline(1, Some(1)).is_err());
        assert!(validate_mainline(0, None).is_ok());
    }
}
