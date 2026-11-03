//! Local ZedX repository features; remote hosts report an explicit capability error.
use super::*;
use git::repository::ShelfEntry;
use uuid::Uuid;

impl Repository {
    pub fn zedx_filter_commits(
        &mut self,
        source: LogSource,
        filter: git::repository::ZedxLogFilter,
    ) -> oneshot::Receiver<Result<Vec<Oid>>> {
        self.send_job("zedx_filter_commits", None, move |state, _| async move {
            match state {
                RepositoryState::Local(state) => state.backend.filter_commits(source, filter).await,
                RepositoryState::Remote(_) => {
                    anyhow::bail!("User, date and path filters are unavailable on this remote host")
                }
            }
        })
    }

    pub fn zedx_shelves(&mut self) -> oneshot::Receiver<Result<Vec<ShelfEntry>>> {
        self.send_job("zedx_shelves", None, move |state, _| async move {
            match state {
                RepositoryState::Local(state) => state.backend.shelves().await,
                RepositoryState::Remote(_) => {
                    anyhow::bail!("Shelf is available for local repositories only")
                }
            }
        })
    }
    pub fn zedx_shelve(&mut self, message: String) -> oneshot::Receiver<Result<ShelfEntry>> {
        self.send_job(
            "zedx_shelve",
            Some("Shelving changes".into()),
            move |state, _| async move {
                match state {
                    RepositoryState::Local(state) => state.backend.shelve(message).await,
                    RepositoryState::Remote(_) => {
                        anyhow::bail!("Shelf is available for local repositories only")
                    }
                }
            },
        )
    }
    pub fn zedx_apply_shelf(&mut self, id: Uuid) -> oneshot::Receiver<Result<()>> {
        self.send_job(
            "zedx_apply_shelf",
            Some("Applying shelf".into()),
            move |state, _| async move {
                match state {
                    RepositoryState::Local(state) => state.backend.apply_shelf(id).await,
                    RepositoryState::Remote(_) => {
                        anyhow::bail!("Shelf is available for local repositories only")
                    }
                }
            },
        )
    }
    pub fn zedx_delete_shelf(&mut self, id: Uuid) -> oneshot::Receiver<Result<()>> {
        self.send_job("zedx_delete_shelf", None, move |state, _| async move {
            match state {
                RepositoryState::Local(state) => state.backend.delete_shelf(id).await,
                RepositoryState::Remote(_) => {
                    anyhow::bail!("Shelf is available for local repositories only")
                }
            }
        })
    }
    pub fn zedx_stash_file_count(&mut self, oid: Oid) -> oneshot::Receiver<Result<usize>> {
        self.send_job("zedx_stash_file_count", None, move |state, _| async move {
            match state {
                RepositoryState::Local(state) => state.backend.stash_file_count(oid).await,
                RepositoryState::Remote(_) => {
                    anyhow::bail!("Stash metadata is unavailable on this remote host")
                }
            }
        })
    }
    pub fn zedx_revert_commit(
        &mut self,
        oid: Oid,
        mainline: Option<usize>,
    ) -> oneshot::Receiver<Result<()>> {
        self.send_job(
            "zedx_revert_commit",
            Some("Reverting commit".into()),
            move |state, _| async move {
                match state {
                    RepositoryState::Local(state) => {
                        state
                            .backend
                            .revert_commit_with_mainline(oid, mainline, state.environment)
                            .await
                    }
                    RepositoryState::Remote(_) => anyhow::bail!(
                        "Revert from Git history is available for local repositories only"
                    ),
                }
            },
        )
    }
}
