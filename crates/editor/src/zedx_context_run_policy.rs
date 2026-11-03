use std::ops::Range;

pub(super) fn matching_rows(
    selection: Range<usize>,
    candidates: &[(u32, Range<usize>)],
) -> Vec<u32> {
    if selection.start > selection.end {
        return Vec::new();
    }
    if !selection.is_empty() {
        let inside = candidates
            .iter()
            .filter(|(_, range)| selection.contains(&range.start))
            .map(|(row, _)| *row)
            .collect::<Vec<_>>();
        if !inside.is_empty() {
            return inside;
        }
    }
    candidates
        .iter()
        .filter(|(_, range)| {
            if selection.is_empty() {
                range.contains(&selection.start)
                    || (range.is_empty() && range.start == selection.start)
            } else {
                range.start <= selection.start && selection.end <= range.end
            }
        })
        .min_by_key(|(row, range)| (range.end.saturating_sub(range.start), *row))
        .map(|(row, _)| vec![*row])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn caret_uses_the_innermost_runnable_and_never_a_neighbour() {
        let tasks = [(0, 0..100), (2, 10..30), (6, 50..70)];
        assert_eq!(matching_rows(20..20, &tasks), [2]);
        assert!(matching_rows(40..40, &tasks[1..]).is_empty());
        assert!(matching_rows(30..30, &tasks[1..]).is_empty());
    }
    #[test]
    fn selection_spanning_two_tests_offers_both_without_the_enclosing_module() {
        assert_eq!(
            matching_rows(10..70, &[(0, 0..100), (2, 10..30), (6, 50..70)]),
            [2, 6]
        );
    }
    #[test]
    fn selected_expression_uses_its_function_and_end_boundary_is_exclusive() {
        let tasks = [(2, 10..30), (6, 50..70)];
        assert_eq!(matching_rows(15..20, &tasks), [2]);
        assert_eq!(matching_rows(10..50, &tasks), [2]);
        assert!(matching_rows(80..90, &tasks).is_empty());
        assert!(matching_rows(20..10, &tasks).is_empty());
    }
}
