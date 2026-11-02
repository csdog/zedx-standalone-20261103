use super::*;
use collections::HashSet;

#[derive(Debug, PartialEq, Eq)]
struct Connection {
    from: usize,
    to: usize,
    bridged: bool,
}

struct Node {
    lane: usize,
    color: usize,
}

pub(super) struct Projection {
    original_count: usize,
    visible: Vec<usize>,
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    lane_count: usize,
}

impl Projection {
    pub(super) fn new(commits: &[Rc<CommitEntry>], visible: &[usize]) -> Self {
        let by_sha = commits
            .iter()
            .enumerate()
            .map(|(index, commit)| (commit.data.sha, index))
            .collect::<HashMap<_, _>>();
        let by_original = visible
            .iter()
            .enumerate()
            .map(|(row, &index)| (index, row))
            .collect::<HashMap<_, _>>();
        let mut lanes = visible
            .iter()
            .filter_map(|&index| commits.get(index).map(|commit| commit.lane))
            .collect::<Vec<_>>();
        lanes.sort_unstable();
        lanes.dedup();
        let lane_positions = lanes
            .iter()
            .enumerate()
            .map(|(position, &lane)| (lane, position))
            .collect::<HashMap<_, _>>();
        let nodes = visible
            .iter()
            .filter_map(|&index| commits.get(index))
            .map(|commit| Node {
                lane: lane_positions[&commit.lane],
                color: commit.color_idx,
            })
            .collect();
        let mut connections = Vec::new();
        for (row, &index) in visible.iter().enumerate() {
            let Some(commit) = commits.get(index) else {
                continue;
            };
            let mut pending = commit
                .data
                .parents
                .iter()
                .copied()
                .map(|sha| (sha, false))
                .collect::<Vec<_>>();
            let mut visited = HashSet::default();
            let mut ancestors = BTreeMap::<usize, bool>::new();
            while let Some((sha, bridged)) = pending.pop() {
                let Some(&parent) = by_sha.get(&sha) else {
                    continue;
                };
                if let Some(&target) = by_original.get(&parent) {
                    if target != row {
                        ancestors
                            .entry(target)
                            .and_modify(|old| *old &= bridged)
                            .or_insert(bridged);
                    }
                } else if visited.insert(parent) {
                    pending.extend(
                        commits[parent]
                            .data
                            .parents
                            .iter()
                            .copied()
                            .map(|sha| (sha, true)),
                    );
                }
            }
            connections.extend(ancestors.into_iter().map(|(to, bridged)| Connection {
                from: row,
                to,
                bridged,
            }));
        }
        Self {
            original_count: commits.len(),
            visible: visible.to_vec(),
            nodes,
            connections,
            lane_count: lanes.len(),
        }
    }

    pub(super) fn matches(&self, original_count: usize, visible: &[usize]) -> bool {
        self.original_count == original_count && self.visible == visible
    }

    pub(super) fn render_row(
        self: &Rc<Self>,
        row: usize,
        row_height: Pixels,
        background: Hsla,
    ) -> AnyElement {
        let projection = self.clone();
        gpui::canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, (), window, cx| {
                window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                    let accents = cx.theme().accents();
                    let x = |node: usize| {
                        bounds.left() + px(8.0 + projection.nodes[node].lane as f32 * 12.0)
                    };
                    for connection in &projection.connections {
                        let from = connection.from as f32;
                        let to = connection.to as f32;
                        let start = from.min(to).max(row as f32 - 0.5);
                        let end = from.max(to).min(row as f32 + 0.5);
                        if start >= end {
                            continue;
                        }
                        let position = |value: f32| {
                            point(
                                x(connection.from)
                                    + (x(connection.to) - x(connection.from))
                                        * ((value - from) / (to - from)),
                                bounds.top() + row_height * (value - row as f32 + 0.5),
                            )
                        };
                        let mut path = PathBuilder::stroke(px(1.5));
                        path.move_to(position(start));
                        path.line_to(position(end));
                        if let Some(path) = path.build().log_err() {
                            let color = accents
                                .color_for_index(projection.nodes[connection.from].color as u32);
                            window.paint_path(
                                path,
                                if connection.bridged {
                                    color.opacity(0.5)
                                } else {
                                    color
                                },
                            );
                        }
                    }
                    if let Some(node) = projection.nodes.get(row) {
                        draw_commit_circle(
                            x(row),
                            bounds.top() + row_height / 2.0,
                            accents.color_for_index(node.color as u32),
                            background,
                            window,
                        );
                    }
                });
            },
        )
        .w(px(16.0 + self.lane_count.saturating_sub(1) as f32 * 12.0))
        .h(row_height)
        .flex_none()
        .into_any_element()
    }
}

impl GitGraph {
    pub(super) fn zedx_graph_projection(&mut self, indices: &[usize]) -> Rc<Projection> {
        if let Some(projection) = &self.zedx_filter.projection {
            if projection.matches(self.graph_data.commits.len(), indices) {
                return projection.clone();
            }
        }
        let projection = Rc::new(Projection::new(&self.graph_data.commits, indices));
        self.zedx_filter.projection = Some(projection.clone());
        projection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commits(parents: &[&[usize]]) -> Vec<Rc<CommitEntry>> {
        let shas = (0..parents.len())
            .map(|index| {
                let mut bytes = [0; 20];
                bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
                Oid::from_bytes(&bytes).unwrap()
            })
            .collect::<Vec<_>>();
        parents
            .iter()
            .enumerate()
            .map(|(index, parents)| {
                Rc::new(CommitEntry {
                    data: Arc::new(InitialGraphCommitData {
                        sha: shas[index],
                        parents: parents.iter().map(|&parent| shas[parent]).collect(),
                        ref_names: Vec::new(),
                    }),
                    lane: index % 2,
                    color_idx: index % 4,
                })
            })
            .collect()
    }

    #[test]
    fn bridges_hidden_ancestors_without_joining_siblings() {
        let commits = commits(&[&[1, 2], &[3], &[3], &[]]);
        let collapsed = Projection::new(&commits, &[0, 3]);
        assert_eq!(
            collapsed.connections,
            vec![Connection {
                from: 0,
                to: 1,
                bridged: true
            }]
        );
        assert!(Projection::new(&commits, &[1, 2]).connections.is_empty());
        let expanded = Projection::new(&commits, &[0, 1, 2, 3]);
        assert_eq!(
            expanded.connections,
            vec![
                Connection {
                    from: 0,
                    to: 1,
                    bridged: false
                },
                Connection {
                    from: 0,
                    to: 2,
                    bridged: false
                },
                Connection {
                    from: 1,
                    to: 3,
                    bridged: false
                },
                Connection {
                    from: 2,
                    to: 3,
                    bridged: false
                },
            ]
        );
        assert_eq!(collapsed.visible, vec![0, 3]);
        assert_eq!(collapsed.nodes[1].color, commits[3].color_idx);
    }

    #[test]
    fn long_hidden_history_uses_an_iterative_walk() {
        let count = 10_000;
        let parents = (0..count)
            .map(|index| {
                if index + 1 < count {
                    vec![index + 1]
                } else {
                    Vec::new()
                }
            })
            .collect::<Vec<_>>();
        let slices = parents.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let commits = commits(&slices);
        let projection = Projection::new(&commits, &[0, count - 1]);
        assert_eq!(
            projection.connections,
            vec![Connection {
                from: 0,
                to: 1,
                bridged: true
            }]
        );
        assert!(projection.matches(count, &[0, count - 1]));
        assert!(!projection.matches(count, &[0, 1]));
    }

    #[test]
    fn malformed_cycles_and_unloaded_parents_do_not_create_false_links() {
        let mut commits = commits(&[&[1], &[0]]);
        let unknown = Oid::from_bytes(&[255; 20]).unwrap();
        Rc::get_mut(&mut commits[0]).unwrap().data = Arc::new(InitialGraphCommitData {
            sha: commits[0].data.sha,
            parents: smallvec![unknown],
            ref_names: Vec::new(),
        });
        assert!(
            Projection::new(&commits, &[0, 1])
                .connections
                .iter()
                .all(|edge| edge.from != 0)
        );
        let cycle = self::commits(&[&[1], &[0]]);
        assert!(Projection::new(&cycle, &[0]).connections.is_empty());
    }
}
