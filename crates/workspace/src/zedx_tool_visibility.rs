use collections::HashMap;
use gpui::{App, BorrowAppContext, Global, Task};
use util::ResultExt;

const KEY: &str = "zedx.tool-window-button-visibility.v1";

#[derive(Default)]
pub(crate) struct ToolVisibility {
    overrides: HashMap<String, bool>,
    persistent: bool,
    write: Option<Task<()>>,
}
impl Global for ToolVisibility {}

pub(crate) fn init(cx: &mut App) {
    let overrides = db::kvp::KeyValueStore::global(cx)
        .read_kvp(KEY)
        .log_err()
        .flatten()
        .and_then(|value| serde_json::from_str(&value).log_err())
        .unwrap_or_default();
    cx.set_global(ToolVisibility {
        overrides,
        persistent: true,
        write: None,
    });
}

pub(crate) fn is_visible(name: &str, default: bool, cx: &App) -> bool {
    cx.try_global::<ToolVisibility>()
        .and_then(|state| state.overrides.get(name))
        .copied()
        .unwrap_or(default)
}

fn save(state: &mut ToolVisibility, cx: &mut App) {
    if !state.persistent {
        return;
    }
    let Some(value) = serde_json::to_string(&state.overrides).log_err() else {
        return;
    };
    let previous = state.write.take();
    let store = db::kvp::KeyValueStore::global(cx);
    state.write = Some(cx.spawn(async move |_| {
        if let Some(previous) = previous {
            previous.await;
        }
        store.write_kvp(KEY.into(), value).await.log_err();
    }));
}

pub(crate) fn toggle(name: &str, default: bool, cx: &mut App) {
    let visible = !is_visible(name, default, cx);
    cx.update_default_global(|state: &mut ToolVisibility, cx| {
        state.overrides.insert(name.into(), visible);
        save(state, cx);
    });
}

pub(crate) fn reset(cx: &mut App) {
    cx.update_default_global(|state: &mut ToolVisibility, cx| {
        state.overrides.clear();
        save(state, cx);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    async fn visibility_persists_explicit_true_and_false_and_resets(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| cx.set_global(db::AppDatabase::test_new()));
        let write = cx.update(|cx| {
            init(cx);
            toggle("Project Panel", true, cx);
            toggle("Branches", false, cx);
            cx.default_global::<ToolVisibility>().write.take().unwrap()
        });
        write.await;
        cx.update(|cx| {
            init(cx);
            assert!(!is_visible("Project Panel", true, cx));
            assert!(is_visible("Branches", false, cx));
            assert!(is_visible("Structure", true, cx));
            reset(cx);
            assert!(is_visible("Project Panel", true, cx));
            assert!(!is_visible("Branches", false, cx));
        });
    }
}
