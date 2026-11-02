use super::*;
use collections::HashSet;

type ActionEntries = Vec<(String, Box<dyn Action>)>;

pub(super) fn collect_actions(window: &Window, cx: &App) -> (ActionEntries, HashSet<usize>) {
    let filter = CommandPaletteFilter::try_global(cx);
    let mut actions = window
        .available_actions(cx)
        .into_iter()
        .filter(|action| !filter.is_some_and(|filter| filter.is_hidden(&**action)))
        .map(|action| (command_palette::humanize_action_name(action.name()), action))
        .collect::<ActionEntries>();
    let mut names = actions
        .iter()
        .map(|(_, action)| action.name())
        .collect::<HashSet<_>>();
    let mut disabled = HashSet::default();
    for name in cx.all_action_names() {
        // Actions requiring parameters cannot be invoked from a plain command row.
        let Ok(action) = cx.build_action(name, None) else {
            continue;
        };
        if filter.is_some_and(|filter| filter.is_hidden(&*action)) || !names.insert(action.name()) {
            continue;
        }
        disabled.insert(actions.len());
        actions.push((command_palette::humanize_action_name(action.name()), action));
    }
    (actions, disabled)
}

impl SearchDelegate {
    pub(super) fn action_is_disabled(&self, target: &Target) -> bool {
        matches!(target, Target::Action(index) if self.disabled_actions.contains(index))
    }

    pub(super) fn scope_includes_extra(&self) -> bool {
        if self.category == Category::Actions {
            self.include_disabled_actions
        } else {
            self.include_non_project
        }
    }

    pub(super) fn set_scope_includes_extra(&mut self, include: bool) {
        if self.category == Category::Actions {
            self.include_disabled_actions = include;
        } else {
            self.include_non_project = include;
        }
    }
}
