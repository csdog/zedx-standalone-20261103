use super::*;
use db::kvp::KeyValueStore;
use gpui::Task;
use serde::{Deserialize, Serialize};

const NAMESPACE: &str = "zedx_project_structure_split";

#[derive(Default)]
pub(super) struct Persistence {
    key: Option<String>,
    modified: bool,
    write: Option<Task<()>>,
}

impl Drop for Persistence {
    fn drop(&mut self) {
        if let Some(write) = self.write.take() {
            write.detach();
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct SavedSplit {
    enabled: bool,
    fraction: Option<f32>,
}

impl SavedSplit {
    fn sanitized(mut self) -> Self {
        self.fraction = self
            .fraction
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(0.05, 0.95));
        self
    }
}

impl Dock {
    pub(crate) fn flush_zedx_split(&mut self, cx: &mut Context<Self>) -> Option<Task<()>> {
        if self.zedx_split.resizing {
            self.zedx_split.resizing = false;
            self.persist_zedx_split(cx);
        }
        let previous = self.zedx_split.persistence.write.take()?;
        let (sender, receiver) = futures::channel::oneshot::channel();
        // Keep the barrier in the write chain: a save started while the caller
        // awaits the flush must still follow every earlier layout write.
        self.zedx_split.persistence.write = Some(cx.background_spawn(async move {
            previous.await;
            if sender.send(()).is_err() {
                log::debug!("Layout flush caller was closed before completion");
            }
        }));
        Some(cx.background_spawn(async move {
            receiver.await.log_err();
        }))
    }

    pub(crate) fn load_zedx_split(
        &mut self,
        workspace: &Workspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.position != DockPosition::Left || !crate::zedx_workbench::enabled() {
            return;
        }
        let Some(key) = workspace
            .database_id()
            .map(|id| i64::from(id).to_string())
            .or(workspace.session_id())
        else {
            return;
        };
        if self.zedx_split.persistence.key.as_ref() != Some(&key) {
            let saved = KeyValueStore::global(cx)
                .scoped(NAMESPACE)
                .read(&key)
                .log_err()
                .flatten()
                .and_then(|json| serde_json::from_str::<SavedSplit>(&json).log_err())
                .map(SavedSplit::sanitized);
            self.zedx_split.persistence.key = Some(key);
            if !self.zedx_split.persistence.modified {
                if let Some(saved) = saved {
                    self.zedx_split.enabled = saved.enabled;
                    self.zedx_split.fraction = saved.fraction;
                }
            } else {
                self.persist_zedx_split(cx);
            }
        }
        self.sync_zedx_split(window, cx);
    }

    pub(super) fn persist_zedx_split(&mut self, cx: &mut Context<Self>) {
        if self.position != DockPosition::Left || !crate::zedx_workbench::enabled() {
            return;
        }
        self.zedx_split.persistence.modified = true;
        let Some(key) = self.zedx_split.persistence.key.clone() else {
            return;
        };
        let saved = SavedSplit {
            enabled: self.zedx_split.enabled,
            fraction: self.zedx_split.fraction,
        }
        .sanitized();
        let Some(json) = serde_json::to_string(&saved).log_err() else {
            return;
        };
        let store = KeyValueStore::global(cx);
        let previous = self.zedx_split.persistence.write.take();
        // A reset or a second drag must not be overwritten by an earlier write.
        self.zedx_split.persistence.write = Some(cx.background_spawn(async move {
            if let Some(previous) = previous {
                previous.await;
            }
            store.scoped(NAMESPACE).write(key, json).await.log_err();
        }));
    }
}

#[cfg(test)]
#[path = "zedx_split_persistence_tests.rs"]
mod tests;
