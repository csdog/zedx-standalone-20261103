use super::*;
use futures::StreamExt as _;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Discovery {
    pub(super) sources: Vec<Source>,
    pub(super) pending: bool,
    pub(super) error: Option<String>,
    pub(super) roots: Vec<PathBuf>,
    task: Option<Task<()>>,
    generation: u64,
}
pub(super) struct Source {
    pub(super) label: String,
    pub(super) worktree: Entity<Worktree>,
    pub(super) prefix: Arc<RelPath>,
    pub(super) absolute: PathBuf,
}
#[derive(Clone)]
struct Locations {
    cargo: PathBuf,
    rustup: PathBuf,
    go_roots: Vec<PathBuf>,
    go_modules: PathBuf,
}
impl Locations {
    fn local() -> Self {
        let home = paths::home_dir();
        let go_path = std::env::var_os("GOPATH")
            .and_then(|paths| std::env::split_paths(&paths).next())
            .unwrap_or_else(|| home.join("go"));
        Self {
            cargo: std::env::var_os("CARGO_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".cargo")),
            rustup: std::env::var_os("RUSTUP_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".rustup")),
            go_roots: std::env::var_os("GOROOT")
                .map(PathBuf::from)
                .into_iter()
                .chain([
                    PathBuf::from("/usr/local/go"),
                    PathBuf::from("/opt/homebrew/opt/go/libexec"),
                ])
                .collect(),
            go_modules: std::env::var_os("GOMODCACHE")
                .map(PathBuf::from)
                .unwrap_or_else(|| go_path.join("pkg/mod")),
        }
    }
}
impl Discovery {
    pub(super) fn reset(&mut self, roots: Vec<PathBuf>) {
        *self = Self {
            roots,
            generation: self.generation + 1,
            ..Default::default()
        };
    }
}

impl ProjectPanel {
    pub(super) fn load_zedx_libraries(&mut self, cx: &mut Context<Self>) {
        if self.zedx_groups.libraries.pending {
            return;
        }
        if !self.project.read(cx).is_local() {
            self.zedx_groups.libraries.error =
                Some("Dependency discovery is available in local projects.".into());
            cx.notify();
            return;
        }
        let project = self.project.clone();
        let roots = project
            .read(cx)
            .visible_worktrees(cx)
            .map(|tree| tree.read(cx).abs_path().to_path_buf())
            .collect::<Vec<_>>();
        self.zedx_groups.libraries.roots = roots.clone();
        let fs = self.fs.clone();
        let locations = Locations::local();
        self.zedx_groups.libraries.pending = true;
        self.zedx_groups.libraries.error = None;
        self.zedx_groups.libraries.generation += 1;
        let generation = self.zedx_groups.libraries.generation;
        self.zedx_groups.libraries.task = Some(cx.spawn(async move |panel, cx| {
            let result: Result<Vec<Source>> = async {
                let candidates = discover(fs, &roots, &locations).await?;
                let mut sources = Vec::new();
                for (absolute, label) in candidates {
                    // The worktree handle retains discovered sources, including ignored
                    // dependency directories inside a visible worktree.
                    let (worktree, prefix) = project.update(cx, |project, cx| project.find_or_create_worktree(&absolute, false, cx)).await?;
                    sources.push(Source { label, worktree, prefix, absolute });
                }
                Ok(sources)
            }.await;
            panel.update(cx, |panel, cx| {
                let current_roots = panel.project.read(cx).visible_worktrees(cx).map(|tree| tree.read(cx).abs_path().to_path_buf()).collect::<Vec<_>>();
                let state = &mut panel.zedx_groups.libraries;
                if state.generation != generation { return; }
                state.pending = false;
                if current_roots != roots {
                    state.sources.clear();
                    state.error = Some("Project folders changed. Refresh Libraries to discover their dependencies.".into());
                } else {
                    match result { Ok(sources) => state.sources = sources, Err(error) => state.error = Some(error.to_string()) }
                }
                cx.notify();
            }).log_err();
        }));
        cx.notify();
    }
}

async fn text(fs: &dyn fs::Fs, path: &Path) -> Result<Option<String>> {
    let Some(metadata) = fs.metadata(path).await? else {
        return Ok(None);
    };
    anyhow::ensure!(
        metadata.len <= 4 * 1024 * 1024,
        "Dependency manifest is too large: {}",
        path.display()
    );
    Ok(Some(fs.load(path).await?))
}
async fn add(
    fs: &dyn fs::Fs,
    found: &mut BTreeMap<PathBuf, String>,
    path: PathBuf,
    label: String,
) -> Result<()> {
    if fs
        .metadata(&path)
        .await?
        .is_some_and(|metadata| metadata.is_dir)
    {
        let canonical = fs.canonicalize(&path).await?;
        found.entry(canonical).or_insert(label);
    }
    Ok(())
}
async fn children(fs: &dyn fs::Fs, path: &Path) -> Result<Vec<PathBuf>> {
    if fs.metadata(path).await?.is_none() {
        return Ok(Vec::new());
    }
    let mut stream = fs.read_dir(path).await?;
    let mut result = Vec::new();
    while let Some(child) = stream.next().await {
        result.push(child?);
        anyhow::ensure!(
            result.len() <= 4096,
            "Too many dependency directories in {}",
            path.display()
        );
    }
    result.sort();
    Ok(result)
}
fn go_cache_name(name: &str) -> String {
    name.chars()
        .flat_map(|character| {
            if character.is_ascii_uppercase() {
                vec!['!', character.to_ascii_lowercase()]
            } else {
                vec![character]
            }
        })
        .collect()
}
async fn discover(
    fs: Arc<dyn fs::Fs>,
    roots: &[PathBuf],
    locations: &Locations,
) -> Result<BTreeMap<PathBuf, String>> {
    let mut found = BTreeMap::new();
    for root in roots {
        if fs.is_file(&root.join("package.json")).await {
            add(
                fs.as_ref(),
                &mut found,
                root.join("node_modules"),
                "Node.js dependencies".into(),
            )
            .await?;
        }
        if fs.is_file(&root.join("go.mod")).await {
            add(
                fs.as_ref(),
                &mut found,
                root.join("vendor"),
                "Go vendor".into(),
            )
            .await?;
            for sdk in &locations.go_roots {
                add(
                    fs.as_ref(),
                    &mut found,
                    sdk.join("src"),
                    "Go SDK sources".into(),
                )
                .await?;
            }
            if let Some(sum) = text(fs.as_ref(), &root.join("go.sum")).await? {
                for line in sum.lines() {
                    let mut fields = line.split_whitespace();
                    let (Some(module), Some(version)) = (fields.next(), fields.next()) else {
                        continue;
                    };
                    let version = version.trim_end_matches("/go.mod");
                    // Validate before joining a manifest-controlled cache path.
                    let relative = format!("{}@{}", go_cache_name(module), go_cache_name(version));
                    RepoRelative::validate(&relative)?;
                    add(
                        fs.as_ref(),
                        &mut found,
                        locations.go_modules.join(relative),
                        format!("Go: {module}@{version}"),
                    )
                    .await?;
                }
            }
        }
        if fs.is_file(&root.join("Cargo.toml")).await {
            add(
                fs.as_ref(),
                &mut found,
                root.join("vendor"),
                "Rust vendor".into(),
            )
            .await?;
            if let Some(lock) = text(fs.as_ref(), &root.join("Cargo.lock")).await? {
                let lock: toml::Value = toml::from_str(&lock)?;
                let buckets = children(fs.as_ref(), &locations.cargo.join("registry/src")).await?;
                for package in lock
                    .get("package")
                    .and_then(toml::Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let Some(source) = package.get("source").and_then(toml::Value::as_str) else {
                        continue;
                    };
                    if !source.starts_with("registry+")
                        || !(source.contains("crates.io-index")
                            || source.contains("index.crates.io"))
                    {
                        continue;
                    }
                    let (Some(name), Some(version)) = (
                        package.get("name").and_then(toml::Value::as_str),
                        package.get("version").and_then(toml::Value::as_str),
                    ) else {
                        continue;
                    };
                    let relative = format!("{name}-{version}");
                    RepoRelative::validate(&relative)?;
                    for bucket in &buckets {
                        if bucket.file_name().is_some_and(|name| {
                            name.to_string_lossy().starts_with("index.crates.io-")
                                || name.to_string_lossy().starts_with("github.com-")
                        }) {
                            add(
                                fs.as_ref(),
                                &mut found,
                                bucket.join(&relative),
                                format!("Rust: {name} {version}"),
                            )
                            .await?;
                        }
                    }
                }
            }
            let override_file = text(fs.as_ref(), &root.join("rust-toolchain.toml")).await?;
            let channel = if let Some(override_file) = override_file {
                let config: toml::Value = toml::from_str(&override_file)?;
                config
                    .get("toolchain")
                    .and_then(|value| value.get("channel"))
                    .and_then(toml::Value::as_str)
                    .map(ToString::to_string)
            } else if let Some(channel) = text(fs.as_ref(), &root.join("rust-toolchain")).await? {
                Some(channel.trim().to_string())
            } else if let Some(settings) =
                text(fs.as_ref(), &locations.rustup.join("settings.toml")).await?
            {
                let settings: toml::Value = toml::from_str(&settings)?;
                settings
                    .get("default_toolchain")
                    .and_then(toml::Value::as_str)
                    .map(ToString::to_string)
            } else {
                None
            };
            if let Some(channel) = channel {
                for installed in children(fs.as_ref(), &locations.rustup.join("toolchains")).await?
                {
                    if installed.file_name().is_some_and(|name| {
                        name == channel.as_str()
                            || name.to_string_lossy().starts_with(&format!("{channel}-"))
                    }) {
                        add(
                            fs.as_ref(),
                            &mut found,
                            installed.join("lib/rustlib/src/rust/library"),
                            format!("Rust SDK: {channel}"),
                        )
                        .await?;
                    }
                }
            }
        }
        if fs.is_file(&root.join("pyproject.toml")).await
            || fs.is_file(&root.join("requirements.txt")).await
            || fs.is_file(&root.join("setup.py")).await
        {
            for environment in [".venv", "venv"] {
                let prefix = root.join(environment);
                add(
                    fs.as_ref(),
                    &mut found,
                    prefix.join("Lib/site-packages"),
                    format!("Python: {environment}"),
                )
                .await?;
                for version in children(fs.as_ref(), &prefix.join("lib")).await? {
                    if version
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with("python"))
                    {
                        add(
                            fs.as_ref(),
                            &mut found,
                            version.clone(),
                            format!("Python SDK: {environment}"),
                        )
                        .await?;
                    }
                }
            }
        }
        anyhow::ensure!(
            found.len() <= 2048,
            "More than 2048 dependency roots were discovered; use a vendor directory to reduce the list"
        );
    }
    Ok(found)
}
struct RepoRelative;
impl RepoRelative {
    fn validate(path: &str) -> Result<()> {
        anyhow::ensure!(
            !Path::new(path).is_absolute()
                && !path
                    .split(['/', '\\'])
                    .any(|part| part == ".." || part.is_empty())
                && !path.contains('\0'),
            "Invalid dependency path in manifest"
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "zedx_library_discovery_tests.rs"]
mod tests;
