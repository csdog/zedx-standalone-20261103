use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZedxLogFilter {
    pub author: String,
    pub after: String,
    pub before: String,
    pub paths: Vec<String>,
}
impl ZedxLogFilter {
    pub fn is_empty(&self) -> bool {
        self.author.is_empty()
            && self.after.is_empty()
            && self.before.is_empty()
            && self.paths.is_empty()
    }
    pub fn validate(&self) -> Result<()> {
        for date in [&self.after, &self.before] {
            if date.is_empty() {
                continue;
            }
            let parts = date.split('-').collect::<Vec<_>>();
            anyhow::ensure!(
                parts.len() == 3
                    && parts[0].len() == 4
                    && parts[1].len() == 2
                    && parts[2].len() == 2,
                "Use dates in YYYY-MM-DD format"
            );
            time::Date::from_calendar_date(
                parts[0].parse()?,
                parts[1].parse::<u8>()?.try_into()?,
                parts[2].parse()?,
            )?;
        }
        anyhow::ensure!(
            self.after.is_empty() || self.before.is_empty() || self.after <= self.before,
            "The start date must not follow the end date"
        );
        anyhow::ensure!(self.paths.len() <= 64, "Use at most 64 paths");
        for path in &self.paths {
            anyhow::ensure!(!path.is_empty(), "Paths must not be empty");
            RepoPath::new(path.as_str())?;
        }
        Ok(())
    }
}
impl RealGitRepository {
    pub(super) fn zedx_filter_commits(
        &self,
        source: LogSource,
        filter: ZedxLogFilter,
    ) -> BoxFuture<'static, Result<Vec<Oid>>> {
        let git = self.git_binary_in_worktree();
        self.executor
            .spawn(async move {
                filter.validate()?;
                anyhow::ensure!(
                    !matches!(source, LogSource::Path(_)) || filter.paths.is_empty(),
                    "Additional path filters are not supported within file history"
                );
                let git = git?;
                let mut args = vec![
                    "log".to_string(),
                    "--format=%H".into(),
                    "--fixed-strings".into(),
                    "--regexp-ignore-case".into(),
                ];
                if !filter.author.is_empty() {
                    args.push(format!("--author={}", filter.author));
                }
                if !filter.after.is_empty() {
                    args.push(format!("--since={} 00:00:00", filter.after));
                }
                if !filter.before.is_empty() {
                    args.push(format!("--until={} 23:59:59", filter.before));
                }
                args.extend(source.get_args().iter().map(ToString::to_string));
                args.extend(filter.paths.iter().map(|path| format!(":(literal){path}")));
                let mut command = git.build_command(&args);
                command.kill_on_drop(true);
                let output = command.output().await?;
                anyhow::ensure!(
                    output.status.success(),
                    "Git log filtering failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                String::from_utf8(output.stdout)?
                    .lines()
                    .map(Oid::from_str)
                    .collect()
            })
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates_and_relative_paths_are_validated() {
        let mut filter = ZedxLogFilter {
            after: "2026-02-30".into(),
            ..Default::default()
        };
        assert!(filter.validate().is_err());
        filter.after = "2026-11-02".into();
        filter.before = "2026-11-01".into();
        assert!(filter.validate().is_err());
        filter.before = "2026-11-02".into();
        filter.paths = vec!["src".into(), "path with spaces".into()];
        assert!(filter.validate().is_ok());
        filter.paths = vec!["../secret".into()];
        assert!(filter.validate().is_err());
    }
}
