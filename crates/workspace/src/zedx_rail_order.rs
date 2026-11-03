use super::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// User-defined rail order, mirroring the prototype's `toolOrder` preference: panels are listed per
/// dock, and anything not listed falls back to the built-in IDEA priority.
#[derive(Default, Clone, Debug)]
pub struct RailOrder {
    slots: HashMap<&'static str, Vec<String>>,
}

impl gpui::Global for RailOrder {}

/// Rail order is only read from and written to disk when the application turned persistence on, so
/// tests never inherit a developer's saved order nor overwrite it.
static PERSISTENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn order_path() -> PathBuf {
    paths::config_dir().join("zedx-rail-order.json")
}

fn dock_key(position: DockPosition) -> &'static str {
    match position {
        DockPosition::Left => "left",
        DockPosition::Bottom => "bottom",
        DockPosition::Right => "right",
    }
}

/// Loads the saved order and enables persistence for this process. Called once from the binary; a
/// test binary never calls it, which keeps the file out of test runs.
pub fn init_global(cx: &mut App) {
    crate::zedx_tool_visibility::init(cx);
    PERSISTENT.store(true, std::sync::atomic::Ordering::Relaxed);
    let order = std::fs::read_to_string(order_path())
        .ok()
        .and_then(|contents| RailOrder::from_json(&contents))
        .unwrap_or_default();
    cx.set_global(order);
}

impl RailOrder {
    pub fn from_json(contents: &str) -> Option<Self> {
        let slots: HashMap<String, Vec<String>> = serde_json::from_str(contents).ok()?;
        Some(Self {
            slots: slots
                .into_iter()
                .filter_map(|(key, value)| {
                    let key = match key.as_str() {
                        "left" => "left",
                        "bottom" => "bottom",
                        "right" => "right",
                        _ => return None,
                    };
                    Some((key, value))
                })
                .collect(),
        })
    }

    fn to_json(&self) -> Option<String> {
        serde_json::to_string(&self.slots).ok()
    }

    fn save(&self) {
        if !PERSISTENT.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let Some(contents) = self.to_json() else {
            return;
        };
        if let Some(parent) = order_path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(order_path(), contents).log_err();
    }

    /// Position of `name` in the saved order, if the user has ever reordered that dock.
    pub fn rank(&self, position: DockPosition, name: &str) -> Option<usize> {
        self.slots
            .get(dock_key(position))?
            .iter()
            .position(|entry| entry == name)
    }

    fn materialise(&mut self, position: DockPosition, current: &[String]) -> &mut Vec<String> {
        let slot = self.slots.entry(dock_key(position)).or_default();
        for entry in current {
            if !slot.contains(entry) {
                slot.push(entry.clone());
            }
        }
        slot.retain(|entry| current.contains(entry));
        slot
    }

    /// Forgets every custom slot, returning each dock to the built-in order. Used by
    /// `Restore Default Layout`, which resets the rail order in the prototype too.
    pub fn reset(&mut self) {
        if self.slots.is_empty() {
            return;
        }
        self.slots.clear();
        self.save();
    }

    /// Moves `name` to `index` in the dock, clamped to the ends. Used by pointer dragging, where the
    /// hovered slot is known directly.
    pub fn move_to(
        &mut self,
        position: DockPosition,
        current: &[String],
        name: &str,
        index: usize,
    ) {
        let slot = self.materialise(position, current);
        let Some(from) = slot.iter().position(|entry| entry == name) else {
            return;
        };
        if slot.is_empty() {
            return;
        }
        let target = index.min(slot.len() - 1);
        if target == from {
            return;
        }
        let entry = slot.remove(from);
        slot.insert(target, entry);
        self.save();
    }

    /// Reorders a fixed group without discarding other panels saved in the same dock.
    pub fn move_subset_to(
        &mut self,
        position: DockPosition,
        current: &[String],
        name: &str,
        index: usize,
    ) {
        let Some(from) = current.iter().position(|entry| entry == name) else {
            return;
        };
        let mut reordered = current.to_vec();
        let entry = reordered.remove(from);
        reordered.insert(index.min(reordered.len()), entry);
        let slot = self.slots.entry(dock_key(position)).or_default();
        for entry in current {
            if !slot.contains(entry) {
                slot.push(entry.clone());
            }
        }
        let mut replacements = reordered.into_iter();
        for entry in slot.iter_mut().filter(|entry| current.contains(entry)) {
            if let Some(replacement) = replacements.next() {
                *entry = replacement;
            }
        }
        self.save();
    }

    /// Moves `name` by `delta` positions. `current` is the dock's panels in the order the user is
    /// looking at, so the first move starts from what is on screen rather than from an empty list.
    pub fn nudge(&mut self, position: DockPosition, current: &[String], name: &str, delta: isize) {
        let slot = self.materialise(position, current);
        let Some(index) = slot.iter().position(|entry| entry == name) else {
            return;
        };
        if slot.is_empty() {
            return;
        }
        let target = (index as isize + delta).clamp(0, slot.len() as isize - 1) as usize;
        if target == index {
            return;
        }
        let entry = slot.remove(index);
        slot.insert(target, entry);
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn first_move_starts_from_the_visible_order() {
        let mut order = RailOrder::default();
        order.nudge(
            DockPosition::Left,
            &names(&["Project Panel", "Find", "Structure"]),
            "Structure",
            -1,
        );
        assert_eq!(order.rank(DockPosition::Left, "Structure"), Some(1));
        assert_eq!(order.rank(DockPosition::Left, "Project Panel"), Some(0));
        assert_eq!(order.rank(DockPosition::Left, "Find"), Some(2));
    }

    #[test]
    fn moves_are_clamped_to_the_ends() {
        let mut order = RailOrder::default();
        let current = names(&["Project Panel", "Find"]);
        order.nudge(DockPosition::Left, &current, "Project Panel", -1);
        assert_eq!(order.rank(DockPosition::Left, "Project Panel"), Some(0));
        order.nudge(DockPosition::Left, &current, "Find", 5);
        assert_eq!(order.rank(DockPosition::Left, "Find"), Some(1));
    }

    #[test]
    fn json_round_trips_and_ignores_unknown_docks() {
        let mut order = RailOrder::default();
        order.nudge(
            DockPosition::Left,
            &names(&["Find", "Structure"]),
            "Structure",
            -1,
        );
        let json = order.to_json().expect("serialisable");
        let restored = RailOrder::from_json(&json).expect("parsable");
        assert_eq!(restored.rank(DockPosition::Left, "Structure"), Some(0));
        assert_eq!(restored.rank(DockPosition::Left, "Find"), Some(1));
        assert!(
            RailOrder::from_json(r#"{"top":["Find"]}"#)
                .expect("parsable")
                .rank(DockPosition::Left, "Find")
                .is_none()
        );
    }

    #[test]
    fn reset_returns_every_dock_to_the_default_order() {
        let mut order = RailOrder::default();
        order.nudge(
            DockPosition::Left,
            &names(&["Find", "Structure"]),
            "Structure",
            -1,
        );
        order.nudge(
            DockPosition::Right,
            &names(&["AgentPanel"]),
            "AgentPanel",
            0,
        );
        assert!(order.rank(DockPosition::Left, "Structure").is_some());
        order.reset();
        assert_eq!(order.rank(DockPosition::Left, "Structure"), None);
        assert_eq!(order.rank(DockPosition::Right, "AgentPanel"), None);
    }

    #[test]
    fn move_to_targets_an_index_and_clamps() {
        let mut order = RailOrder::default();
        let current = names(&["Project Panel", "Find", "Structure"]);
        order.move_to(DockPosition::Left, &current, "Structure", 0);
        assert_eq!(order.rank(DockPosition::Left, "Structure"), Some(0));
        assert_eq!(order.rank(DockPosition::Left, "Project Panel"), Some(1));
        order.move_to(DockPosition::Left, &current, "Structure", 99);
        assert_eq!(
            order.rank(DockPosition::Left, "Structure"),
            Some(2),
            "an out-of-range index clamps to the last slot"
        );
    }

    #[test]
    fn reordering_fixed_controls_preserves_extra_panels_and_round_trips() {
        let mut order = RailOrder::from_json(
            r#"{"right":["AgentPanel","Project Panel","Outline Panel","Database"]}"#,
        )
        .unwrap();
        order.move_subset_to(
            DockPosition::Right,
            &names(&["AgentPanel", "Outline Panel", "Database"]),
            "Database",
            0,
        );
        let restored = RailOrder::from_json(&order.to_json().unwrap()).unwrap();
        for (index, name) in ["Database", "Project Panel", "AgentPanel", "Outline Panel"]
            .iter()
            .enumerate()
        {
            assert_eq!(restored.rank(DockPosition::Right, name), Some(index));
        }
    }

    #[test]
    fn docks_are_independent() {
        let mut order = RailOrder::default();
        order.nudge(
            DockPosition::Left,
            &names(&["Find", "Structure"]),
            "Structure",
            -1,
        );
        assert_eq!(order.rank(DockPosition::Right, "Structure"), None);
        assert_eq!(
            order.rank(DockPosition::Bottom, "Structure"),
            None,
            "a reorder must not leak into another dock"
        );
    }
}
