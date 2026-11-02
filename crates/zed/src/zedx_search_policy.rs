#[derive(Clone, Copy, Default, PartialEq)]
pub(super) enum FileFilter {
    #[default]
    All,
    Rust,
    Configuration,
    Markdown,
    Other,
}

impl FileFilter {
    pub(super) const ALL: [Self; 5] = [
        Self::All,
        Self::Rust,
        Self::Configuration,
        Self::Markdown,
        Self::Other,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "All file types",
            Self::Rust => "Rust · .rs",
            Self::Configuration => "Configuration · .toml .json .yaml",
            Self::Markdown => "Markdown · .md",
            Self::Other => "Other file types",
        }
    }

    pub(super) fn accepts(self, path: &str) -> bool {
        let extension = path.rsplit('.').next().unwrap_or_default();
        match self {
            Self::All => true,
            Self::Rust => extension.eq_ignore_ascii_case("rs"),
            Self::Configuration => ["toml", "json", "jsonc", "yaml", "yml"]
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate)),
            Self::Markdown => ["md", "markdown"]
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate)),
            Self::Other => ![Self::Rust, Self::Configuration, Self::Markdown]
                .iter()
                .any(|filter| filter.accepts(path)),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct FileTypes(u8);

impl Default for FileTypes {
    fn default() -> Self {
        Self(15)
    }
}

impl FileTypes {
    pub(super) fn only(filter: FileFilter) -> Self {
        Self(match filter {
            FileFilter::All => 15,
            FileFilter::Rust => 1,
            FileFilter::Configuration => 2,
            FileFilter::Markdown => 4,
            FileFilter::Other => 8,
        })
    }
    pub(super) fn is_all(self) -> bool {
        self.0 == 15
    }
    pub(super) fn contains(self, filter: FileFilter) -> bool {
        let bits = Self::only(filter).0;
        self.0 & bits == bits
    }
    pub(super) fn toggle(&mut self, filter: FileFilter) {
        if filter == FileFilter::All {
            self.0 = if self.is_all() { 0 } else { 15 };
        } else {
            self.0 ^= Self::only(filter).0;
        }
    }
    pub(super) fn accepts(self, path: &str) -> bool {
        FileFilter::ALL
            .into_iter()
            .filter(|filter| *filter != FileFilter::All)
            .any(|filter| self.contains(filter) && filter.accepts(path))
    }
    pub(super) fn label(self) -> String {
        if self.is_all() {
            return "All file types".into();
        }
        if self.0 == 0 {
            return "No file types selected".into();
        }
        FileFilter::ALL
            .into_iter()
            .filter(|filter| *filter != FileFilter::All && self.contains(*filter))
            .map(FileFilter::label)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub(super) fn highlight_indices(label: &str, query: &str) -> Vec<usize> {
    let query: Vec<_> = query
        .trim()
        .chars()
        .map(|character| character.to_lowercase().collect::<String>())
        .collect();
    if query.is_empty() {
        return Vec::new();
    }
    let characters: Vec<_> = label
        .char_indices()
        .map(|(offset, character)| (offset, character.to_lowercase().collect::<String>()))
        .collect();
    if let Some(start) = characters.windows(query.len()).position(|window| {
        window
            .iter()
            .zip(&query)
            .all(|((_, character), expected)| character == expected)
    }) {
        return characters[start..start + query.len()]
            .iter()
            .map(|(offset, _)| *offset)
            .collect();
    }
    let mut positions = Vec::new();
    let mut next = 0;
    for (offset, character) in characters {
        if query.get(next) == Some(&character) {
            positions.push(offset);
            next += 1;
        }
    }
    if next == query.len() {
        positions
    } else {
        Vec::new()
    }
}

pub(super) fn action_search_text(name: &str, display: &str) -> String {
    let mut aliases = match name {
        "zedx::ToggleCommit" => "提交 变更 git commit changes",
        "zedx::RestoreDefaultLayout" => "恢复布局 重置布局 默认布局 面板 reset layout",
        "zedx::FindInFiles" => "搜索 查找 全局 内容 Find in Files",
        "zedx::ToggleStructure" => "结构 符号 outline structure",
        "zedx::ToggleProblems" => "问题 诊断 错误 警告 problems diagnostics",
        "zedx::ToggleRun" => "运行 任务 run tasks",
        "zedx::ToggleBranches" => "分支 切换分支 branches git",
        "zedx::ShowGitHistory" => "历史 提交记录 日志 log history commits",
        "zedx::ToggleTheme" => "主题 外观 深色 浅色 切换",
        "zed::OpenSettings" => "设置 偏好 配置",
        "pane::DeploySearch" => "搜索 查找 全局 内容",
        "workspace::ToggleLeftDock" => "项目 侧栏 左边栏",
        "terminal_panel::ToggleFocus" => "终端 命令行",
        _ => "",
    }
    .to_string();
    if name.starts_with("pane::Close") {
        aliases.push_str(" 关闭 标签");
    }
    if name.starts_with("pane::Split") {
        aliases.push_str(" 拆分 分屏");
    }
    if name.starts_with("workspace::Toggle") {
        aliases.push_str(" 切换 面板");
    }
    if name.starts_with("workspace::Save") {
        aliases.push_str(" 保存");
    }
    if name.starts_with("workspace::New") {
        aliases.push_str(" 新建");
    }
    if name.starts_with("workspace::ActivatePane") {
        aliases.push_str(" 焦点 窗格");
    }
    if name.starts_with("multi_workspace::") {
        aliases.push_str(" 项目 工作区");
    }
    if name == "theme::ToggleMode" {
        aliases.push_str(" 主题 外观");
    }
    format!("{display} {aliases}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_uses_extension_not_parent_or_basename_fragments() {
        assert!(FileFilter::Rust.accepts("nested/a.RS"));
        assert!(!FileFilter::Rust.accepts("src.rs/readme.md"));
        assert!(!FileFilter::Rust.accepts("src/main.rs.bak"));
        assert!(FileFilter::Configuration.accepts("Cargo.toml"));
        assert!(FileFilter::Configuration.accepts("settings.JSONC"));
        assert!(!FileFilter::Configuration.accepts("Cargo.lock"));
        assert!(FileFilter::Markdown.accepts("README.md"));
        assert!(FileFilter::All.accepts("LICENSE"));
    }

    #[test]
    fn chinese_aliases_keep_display_name_and_stable_action_identity() {
        let search = action_search_text("zedx::ToggleTheme", "Zedx: Toggle Theme");
        assert!(search.contains("主题"));
        assert!(search.starts_with("Zedx: Toggle Theme"));
        assert!(!action_search_text("unknown::Theme", "Other").contains("主题"));
        let close = action_search_text("pane::CloseActiveItem", "Pane: Close Active Item");
        assert!(close.starts_with("Pane: Close Active Item"));
        assert!(close.contains("关闭"));
        assert!(action_search_text("pane::SplitRight", "Pane: Split Right").contains("拆分"));
        assert!(
            action_search_text(
                "workspace::ToggleBottomDock",
                "Workspace: Toggle Bottom Dock"
            )
            .contains("切换")
        );
        assert!(action_search_text("workspace::Save", "Workspace: Save").contains("保存"));
        let branches = action_search_text("zedx::ToggleBranches", "Zedx: Toggle Branches");
        assert!(branches.contains("分支") && branches.contains("branches"));
        let history = action_search_text("zedx::ShowGitHistory", "Zedx: Show Git History");
        assert!(history.contains("历史") && history.contains("提交记录"));
        let multi = action_search_text("multi_workspace::CloseAll", "Multi Workspace: Close All");
        assert!(multi.contains("项目") && multi.contains("工作区"));
    }

    #[test]
    fn every_filter_has_a_distinct_label_and_all_accepts_extensionless_files() {
        let labels: std::collections::HashSet<_> = FileFilter::ALL
            .iter()
            .map(|filter| filter.label())
            .collect();
        assert_eq!(labels.len(), FileFilter::ALL.len());
        assert!(FileFilter::default().accepts("Makefile"));
    }
}

#[cfg(test)]
mod multi_select_tests {
    use super::*;
    #[test]
    fn types_are_independent_and_an_empty_selection_matches_nothing() {
        let mut types = FileTypes::default();
        types.toggle(FileFilter::All);
        assert!(!types.accepts("main.rs"));
        assert_eq!(types.label(), "No file types selected");
        types.toggle(FileFilter::Rust);
        types.toggle(FileFilter::Markdown);
        assert!(types.accepts("main.rs"));
        assert!(types.accepts("README.md"));
        assert!(!types.accepts("Cargo.toml"));
        types.toggle(FileFilter::Rust);
        assert!(!types.accepts("main.rs"));
        assert!(types.accepts("README.md"));
        types.toggle(FileFilter::All);
        assert!(types.is_all());
        assert!(types.accepts("LICENSE"));
    }
    #[test]
    fn exact_match_is_preferred_to_an_earlier_subsequence() {
        assert_eq!(highlight_indices("a_b ab", "ab"), vec![4, 5]);
        assert_eq!(
            highlight_indices("ToolWindowBar", "twwb"),
            vec![0, 4, 9, 10]
        );
        assert!(highlight_indices("main.rs", "missing").is_empty());
    }
    #[test]
    fn unicode_highlights_are_original_utf8_boundaries() {
        let label = "主题 İcon 大纲";
        for query in ["主题", "大纲", "İc", "主纲", ""] {
            let indices = highlight_indices(label, query);
            assert!(indices.iter().all(|index| label.is_char_boundary(*index)));
        }
        assert_eq!(highlight_indices(label, "主题"), vec![0, 3]);
        assert_eq!(highlight_indices(label, "大纲"), vec![13, 16]);
    }
}
