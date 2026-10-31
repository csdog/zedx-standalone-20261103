use std::time::Duration;

#[derive(Default)]
pub(super) struct DoubleShift {
    pressed_at: Option<Duration>,
    tapped_at: Option<Duration>,
    shift_down: bool,
}

impl DoubleShift {
    pub(super) fn reset(&mut self) {
        self.pressed_at = None;
        self.tapped_at = None;
    }

    pub(super) fn modifiers_changed(
        &mut self,
        shift: bool,
        another_modifier: bool,
        now: Duration,
    ) -> bool {
        let was_down = self.shift_down;
        self.shift_down = shift;
        if another_modifier {
            self.reset();
            return false;
        }
        if shift && !was_down {
            self.pressed_at = Some(now);
        } else if !shift && was_down {
            let valid_tap = self
                .pressed_at
                .take()
                .is_some_and(|pressed| now.saturating_sub(pressed) <= Duration::from_millis(350));
            if !valid_tap {
                self.tapped_at = None;
                return false;
            }
            if self
                .tapped_at
                .take()
                .is_some_and(|tapped| now.saturating_sub(tapped) <= Duration::from_millis(400))
            {
                return true;
            }
            self.tapped_at = Some(now);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(milliseconds: u64) -> Duration {
        Duration::from_millis(milliseconds)
    }

    #[test]
    fn two_short_shift_taps_open_search() {
        let mut gesture = DoubleShift::default();
        assert!(!gesture.modifiers_changed(true, false, at(0)));
        assert!(!gesture.modifiers_changed(false, false, at(80)));
        assert!(!gesture.modifiers_changed(true, false, at(180)));
        assert!(gesture.modifiers_changed(false, false, at(250)));
    }

    #[test]
    fn uppercase_typing_between_taps_does_not_open_search() {
        let mut gesture = DoubleShift::default();
        gesture.modifiers_changed(true, false, at(0));
        gesture.reset();
        gesture.modifiers_changed(false, false, at(80));
        gesture.modifiers_changed(true, false, at(180));
        assert!(!gesture.modifiers_changed(false, false, at(250)));
    }

    #[test]
    fn modifier_chords_do_not_count_as_taps() {
        let mut gesture = DoubleShift::default();
        gesture.modifiers_changed(true, true, at(0));
        gesture.modifiers_changed(false, false, at(80));
        gesture.modifiers_changed(true, false, at(180));
        assert!(!gesture.modifiers_changed(false, false, at(250)));
    }

    #[test]
    fn held_shift_does_not_count_as_a_tap() {
        let mut gesture = DoubleShift::default();
        gesture.modifiers_changed(true, false, at(0));
        gesture.modifiers_changed(false, false, at(500));
        gesture.modifiers_changed(true, false, at(600));
        assert!(!gesture.modifiers_changed(false, false, at(650)));
    }

    #[test]
    fn slow_taps_do_not_open_search() {
        let mut gesture = DoubleShift::default();
        gesture.modifiers_changed(true, false, at(0));
        gesture.modifiers_changed(false, false, at(80));
        gesture.modifiers_changed(true, false, at(700));
        assert!(!gesture.modifiers_changed(false, false, at(750)));
    }
}
