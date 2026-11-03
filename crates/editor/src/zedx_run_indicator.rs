use super::RunnableTaskStatus;
use ui::{Color, IconName, IconSize};

pub(super) fn appearance(
    status: Option<RunnableTaskStatus>,
    zedx: bool,
    light: bool,
) -> (IconName, Color, IconSize) {
    let (icon, color) = match status {
        Some(RunnableTaskStatus::Passed) => (IconName::Check, Color::Success),
        Some(RunnableTaskStatus::Failed) => (IconName::XCircle, Color::Error),
        Some(RunnableTaskStatus::Running) if !zedx => (IconName::PlayOutlined, Color::Accent),
        None if !zedx => (IconName::PlayOutlined, Color::Muted),
        _ => (
            IconName::ZedxRun,
            Color::Custom(gpui::rgb(if light { 0x369650 } else { 0x59a869 }).into()),
        ),
    };
    (
        icon,
        color,
        if zedx {
            IconSize::Medium
        } else {
            IconSize::XSmall
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_and_running_use_large_green_outline_triangle_in_both_themes() {
        for light in [true, false] {
            for status in [None, Some(RunnableTaskStatus::Running)] {
                let (icon, color, size) = appearance(status, true, light);
                assert_eq!(icon, IconName::ZedxRun);
                assert!(matches!(size, IconSize::Medium));
                assert_eq!(
                    color,
                    Color::Custom(gpui::rgb(if light { 0x369650 } else { 0x59a869 }).into())
                );
            }
        }
    }
    #[test]
    fn results_keep_success_and_failure_meaning() {
        let (passed, success, _) = appearance(Some(RunnableTaskStatus::Passed), true, false);
        assert_eq!(passed, IconName::Check);
        assert_eq!(success, Color::Success);
        let (failed, error, _) = appearance(Some(RunnableTaskStatus::Failed), true, false);
        assert_eq!(failed, IconName::XCircle);
        assert_eq!(error, Color::Error);
    }
    #[test]
    fn upstream_mode_retains_original_outline_size_and_colors() {
        let (icon, color, size) = appearance(None, false, false);
        assert_eq!(icon, IconName::PlayOutlined);
        assert_eq!(color, Color::Muted);
        assert!(matches!(size, IconSize::XSmall));
        assert_eq!(
            appearance(Some(RunnableTaskStatus::Running), false, false).1,
            Color::Accent
        );
    }
}
