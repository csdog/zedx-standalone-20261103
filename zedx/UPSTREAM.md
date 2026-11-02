# Upstream integration

Base: `zed-industries/zed` **v1.26.2**, commit `c4273271092b0287f7d181f616bd5171e0a2f70c`.
Remote: `upstream` → `https://github.com/zed-industries/zed.git`.
The imported baseline is the current HEAD; local fork changes are uncommitted.

## Replacement modules

| New file | Responsibility | Small upstream entry point |
|---|---|---|
| `crates/workspace/src/zedx_workbench.rs` | IDEA-style frame and tool window rails | `workspace.rs`: module, panel-button entities, layout match arm, drag offsets |
| `crates/workspace/src/zedx_tool_window_bar.rs` | Vertical real Panel buttons | `dock.rs`: child module and rendering branch |
| `crates/title_bar/src/zedx_project_identity.rs` | Project initials avatar and existing recent-project menu | `title_bar.rs`: module and one render call |
| `crates/release_channel/src/zedx_brand.rs` | Fork identity constants | `release_channel/src/lib.rs`: name, application ID, disable official updates |
| `crates/zed/src/zedx_search.rs` | Native Search Everywhere modal and real search providers | `main.rs`: module and init call; `zed.rs`: one default-keymap reload hook |
| `crates/zed/src/zedx_gesture.rs` | Pure double-Shift state machine and tests | Included only by ZedX search |
| `crates/workspace/src/zedx_compact.rs` | Compact dock mutual exclusion and file-open dismissal | `workspace.rs`: module declaration; owned actions initialize observers |
| `crates/workspace/src/zedx_layout.rs` / `zedx_bottom_bar.rs` | Restore default dock sizes and bottom tool-window tabs/clear/hide | `workspace.rs`: two module declarations; owned workbench calls the header |
| `crates/zed/src/zedx_panels.rs` / `zedx_run_panel.rs` | Left Structure, live Problems, real task-terminal Run view | `main.rs`: module/init; `zed.rs`: one joined initialization future |
| `crates/zed/src/zedx_status_bar.rs` | Real branch/save state and current-file indentation | `main.rs`: module; `zed.rs`: status item registration/order |
| `script/zedx` / `zedx/dev_bundle.py` | Fork development entry point and local macOS bundle | No upstream script replacement |

Additional seams: `assets/settings/default.json` changes only the project-panel default dock to left; `paths.rs` changes the documented fork APP_NAME constant; `crates/zed/Cargo.toml` changes binary/default-run and adds existing workspace dependencies; Cargo.lock records those dependencies. README retains the upstream manual-review marker. `.gitignore` preserves prototype/local rules.

## Updating

1. Keep the working tree saved before fetching a new upstream tag.
2. The initial import is shallow. Before the first history merge, run `git fetch --unshallow upstream`. Fetch the desired stable tag from `upstream`, review its release notes and diff from the recorded base.
3. Merge into a dedicated local integration branch. Resolve the small entry points above; do not transplant the entire upstream workspace/title-bar implementation.
4. Run `script/zedx check`, `script/zedx gesture-test`, the upstream `script/clippy` for affected packages, and native interaction checks.
5. Compare `script/zedx run .` with `script/zedx upstream-ui .`; both retain ZedX storage and disabled official updater.
6. Update this baseline and `zedx/upstream.json` only after validation. Commit/push only when requested.

The native frame currently uses the contained bottom-dock layout. The legacy layout remains available with `ZEDX_UPSTREAM_UI=1`; it is an upstream comparison aid. Existing editor/project/language-server behavior is reused.

## Licensing

The imported upstream license files are preserved. Prototype icon attributions remain in `THIRD_PARTY_NOTICES.md` and `docs/icons-used.md`. Native icon mapping uses the attributed IntelliJ-style assets in `assets/icons/zedx`; the theme enables the existing GPUI blurred background material.


## Native validation (2026-10-31)

- Unmodified stable baseline: `cargo check -p workspace --locked` passed.
- Native binary: `script/zedx build` passed; `target/debug/ZedX.app` launched and rendered.
- Strict lint: `script/clippy -p zed -p workspace -p title_bar -p release_channel --locked` passed, including release/all-targets/all-features.
- `script/zedx gesture-test`: 5 passed (double tap, uppercase typing, modifier chord, long hold, slow taps).
- Native UI checks in `/private/tmp/zedx-smoke`: project initials, left project tree, file result/open, Cmd+K from the editor, Tab category cycling, real command results, text results with path/line, Enter navigation to the highlighted match all verified.
- The text search task handle must remain alive while receiving results. Do not capture only `SearchResults.rx`, which would cancel the producer before it yields matches.
- At the first slice, native double-Shift dispatch awaited physical keyboard verification (automation rejects modifier-only presses), and the smoke project stayed in Restricted Mode. Subsequent native LSP and GPUI event evidence is recorded below.
- Debug linking reports the upstream large `__eh_frame` warning; the upstream `block 0.1.6` dependency reports a future Rust compatibility warning. Neither prevented the build or launch.

The first slice was not the finished redesign. Subsequent icon/material work is recorded below; complete onboarding/packaging branding, signed distribution and an upstream-update rehearsal remain future work. Prototype completion is tracked separately in PROTOTYPE_PARITY.md.


## Prototype parity work (2026-10-31)

- Full remaining scope is tracked in `zedx/PROTOTYPE_PARITY.md`; the goal remains complete prototype implementation and tests.
- Added `assets/themes/zedx/zedx.json`: paired native themes, system blur, translucent frame tokens, opaque editor and panel surfaces, prototype palette. One themes remain intact.
- Added 39 light/dark glyph pairs under `assets/icons/zedx/`, exported by `zedx/export_icons.py` with `zedx/icon-manifest.json`. SVG intrinsic dimensions are 4× viewBox dimensions because GPUI's multicolor image decoder rasterizes at scale 1.
- Added `crates/ui/src/zedx.rs` / `zedx_island.rs` and `crates/file_icons/src/zedx_file_icons.rs`. Small seams: UI module export, embedded ZedX image branch in Icon rendering, file/folder/chevron lookup hooks. Other icon paths keep upstream behavior.
- Added shared `workspace::zedx_actions` and `zedx_title_actions.rs`; Search Everywhere action types moved to the shared module so title bar and binary use identical actions. The title-bar seam adds the action group. No dependency on the binary is introduced.
- Default settings now select ZedX theme variants and 13px UI/code fonts. Project/Git default left, Agent default right; project width 265, outline/agent width 270, terminal height 202. Existing explicit preferences and saved sizes remain authoritative.
- Workbench adds 6px vertical inset and independent corner painting: GPUI masks children to rectangles, so a rounded background alone cannot round editor/panel children. Bottom drag math subtracts the new inset; legacy upstream mode remains at zero inset.
- Native runtime verified both ZedX appearances and colored file icons; theme survives restart. Rust analyzer fixture classes/struct search navigated to row 1 and function symbol search to row 5. Task configuration spawned the real fixture and produced `Hello ZedX Search Everywhere`; Debug opened the real debug selector.
- `script/zedx assets-test`: 5 tests passed (asset XML/light-dark/retina coverage, enum-manifest mapping, material alpha/opaque editor, actual JSON panel defaults, theme registration references). The five gesture tests still pass. Native build and strict Clippy for zed/workspace/title_bar/ui/file_icons passed.


## Search and resize continuation (2026-10-31)

- Added `zedx_search_policy.rs` (file categories and stable Chinese action aliases), `zedx_search_paths.rs` (filter before fuzzy top-N ranking) and `zedx_search_tests.rs` (GPUI integration). These are owned fork files; upstream picker/project/GPUI implementations remain untouched.
- Shared `zedx_actions.rs` adds SearchFiles; the existing keymap seam now binds Cmd/Ctrl+P. Repeated Search Everywhere changes scope instead of closing the modal.
- Search adds actual provider scope, file filters, query clearing, Find in Files, right split and selected source footer. All retains a separate streaming text task alongside path/symbol/action providers. Generation checks and picker task cancellation prevent old queries publishing into new results.
- Workbench gap handles reuse upstream DraggedDock, resizing, reset, serialization and size persistence. They add focused arrow keys (8px), Shift+arrow (24px), Home reset and double-click reset in the independent module. Debug selectors make hit regions testable without changing GPUI.
- GPUI tests use a real MultiWorkspace root, which mounts Workspace action renderers. A bare Workspace root does not represent the production event dispatch tree. Gesture time uses GPUI's executor clock so event timing is deterministic in tests.
- Six GPUI integration cases passed: modifier opening/repeat scope; Cmd+P and Shift+Enter split; All text/type filtering; gap drag/reset/keyboard; ignored file/text scope plus stale query cancellation; filtering with 151 file candidates. Three policy, five gesture and five asset tests also passed.
- Native app checks: Cmd+P Files, README opened in the right split, All merged two real text matches plus an action, Rust filter removed Markdown text, synchronized All Places checkbox, Chinese 主题 executed the theme switch, Find in Files opened actual project results with the query preserved. Physical drag automation did not resize the window; GPUI mouse events verified drag logic, and further physical/restart dock checks remain.

- Final checks after the continuation: `script/zedx check`, `build`, `integration-test` (6), `search-test` (3), `gesture-test` (5), `assets-test` (5) passed. Strict `script/clippy -p zed -p workspace -p title_bar -p ui -p file_icons --locked` passed with release/all-targets/all-features and warnings denied. Only the known upstream debug-link/future-compatibility warnings remain on build/check.
- Latest native run also verified left width 265 -> 281 with two Right presses, retention at 281 after restart, Home reset to 265, bottom height 202 -> 218 with two Up presses and Home reset. App remains open with the latest build for review.


## Grouped search and compact workbench (2026-10-31)

- New `crates/zed/src/zedx_search_view.rs` owns the search chrome, grouped variable-height rows, exact/subsequence highlight labels, multi-select filter popover, compact footer and responsive transparent modal. The backend stays in `zedx_search.rs`; upstream list and GPUI source are not copied.
- New `crates/picker/src/zedx_embedded.rs` fits only the ZedX embedded search and refreshes streaming list counts/preview without dropping pending producer tasks. Single upstream seam: `mod zedx_embedded;` in picker.rs. The standard picker sizing path remains intact. Hidden preview height caps the list rather than the entire picker, so the fork subtracts query/footer chrome when budgeting that cap.
- Multi-select state and UTF-8 byte-safe exact/subsequence highlighting live in the independent search policy module. File candidates are filtered before fuzzy top-N ranking. An empty type selection matches no files/symbols/text; actions remain independent.
- All/Actions accepts the prototype `/` command prefix. Sections sort by class/file/symbol/action/text, preserve target selection when providers append, and show symbol file/line metadata.
- Workbench uses deferred absolute 235px overlays below 560px, preserving desktop dock widths. Title actions hide configuration text below 760px; project names shorten below 560px; the small title-bar seams hide branch/sign-in below 760px.
- Ten GPUI integration tests pass, including grouped streaming rendering, multi-select/none, adaptive height, 60 results in four viewports with preview, footer visibility, keyboard wrap/reverse categories, Ctrl+P, disabled Actions->Find, slash command mode, and compact-overlay -> desktop restoration. Six pure policy/highlight tests, five gesture tests and five asset tests pass (26 total).
- Native evidence: desktop Actions/Text headings and highlights, persistent multi-select menu with Rust+Markdown, clear-to-recent, real lower code preview and selected source footer. Native window resized via macOS Window -> Move & Resize -> Top Left, then dragging its actual bottom-right edge to ~392px width. The project overlay and compact top actions work; native Ctrl+P shows a long scrollable file list with an unclipped footer. Physical dock-drag automation still differs from OS window resizing and remains independently tracked.
- Remaining scope stays in PROTOTYPE_PARITY.md, including original tool-window/status-bar parity, missing Restore Default Layout, and broader native compact/light/overlay edge cases.

- Final native follow-up verified `/` support by GPUI test, plus the latest compact 27px rows / 280px list cap / 175px preview and 7vh placement. A ~392×485 native window passed light and dark file search with source preview, complete footer and preview-state retention after reopening. macOS Fill restored the desktop view; Project restored to 265px. The final app is left open with grouped real text matches and source preview.
- Final `script/zedx build` and strict Clippy for zed/workspace/title_bar/picker passed after the streaming-list and height fixes; `search-test` (6), `gesture-test` (5), `assets-test` (5), `integration-test` (10) all passed. Additional upstream seams in this round: title-bar render passes Window into the owned action renderer and gates branch/sign-in text by width; all substantive logic remains in owned modules.

## Tool windows and status (2026-10-31)

- Main-menu action search, Restore Default Layout and Chinese aliases are registered through owned action modules. Database and Notifications retain the prototype placeholder behavior.
- Structure embeds a second upstream OutlinePanel, forwarding its PanelEvents; Problems subscribes to Project diagnostics, loads source buffers and navigates real anchor ranges; Run renders the task TerminalView owned by the upstream terminal panel. No duplicate task process is launched.
- The bottom header lists actual Terminal/Run/Problems/Debug panels, dispatches terminal Clear and hides the dock. ZedX disables upstream nested resize handles through one conditional in `dock.rs`; the original mode keeps them.
- Status items read GitStore/Repository, real item dirty state and current buffer language settings. Encoding/line-ending defaults enable the existing native indicators.
- `script/zedx integration-test` now includes all `zedx_` tests: 26 passed, including 5 new GPUI regressions for panel toggles, reset sizes/visibility, diagnostic display/navigation/removal and save/indentation state. Production changes remain in owned modules; shared dependency additions (`lsp`, `terminal`, `task`) use existing workspace packages.
- Remaining limitations: layout reset restores sizes/visibility/active panels but does not relocate a panel moved to another dock; Structure shares upstream outline preferences; task hide/reveal edge cases and native visual parity require continued verification. The full remaining matrix is tracked in PROTOTYPE_PARITY.md.

- Final native follow-up: Structure displays real Rust symbols; `cargo run` auto-selects Run with stdout and completion status; a temporary `i32`/string mismatch appears in Problems and clicking selects the exact range. Restoring the fixture clears the list. Restore Default Layout selects Project/Terminal; clear-console removes output and Hide closes the bottom dock. The natural Chinese phrase `恢复布局` alias was corrected after a native mismatch and is covered by a dedicated GPUI regression.
- Current validation: build/check passed; strict Clippy for zed/workspace/title_bar/ui passed (release, all-targets/all-features, warnings denied); 26 Rust tests plus 5 asset tests pass. Known upstream debug-link and block dependency compatibility warnings remain.

## Compact interaction and Git status continuation (2026-10-31)

- `zedx_compact.rs` owns compact dock observers, resize reconciliation and file/preview dismissal. Original workspace adds one module declaration; owned action initialization attaches the observers. Dock sizing storage is preserved.
- Owned tool-window rail sorts Project / Find / Git / Structure, Run / Terminal, and Agent / Outline before extra upstream tools. Find insertion and ordering are confined to the existing ZedX dock child module.
- Status now subscribes to GitStoreEvent rather than relying on generic entity notifications: adding Git after opening the window previously left the status at Local project. A regression reproduced the failure and passed after the fix; native symbolic HEAD changes also refresh the real branch label.
- A disposable `/private/tmp/zedx-smoke` Git repository, without commits/remotes, exercises actual diff and index changes. Native stage changed README from AM to A; native unstage changed it to ??; CLI status verified both. No commit/push occurred.
- ZedX notification rendering is an owned helper selected by a one-line call replacement in workspace. Deferred notifications paint above compact docks and clamp to viewport width; original notification rendering remains available for upstream UI. GPUI verifies a notification button beneath the dock bounds receives the click.

- Final checks for this continuation: `script/zedx check`, `build`, `integration-test` (30), `assets-test` (5), `gesture-test` (5, already included in Rust total), strict `script/clippy -p zed -p workspace --locked` and `git diff --check` passed.
- Final native ~392px follow-up: Project/Git -> right Outline -> Project switches show one overlay at a time. Single-click README preview closes Project; Right moves the real editor cursor 1:1 -> 1:2, verifying keyboard focus. Database and Notifications messages render fully above Project and auto-hide. Native status shows real symbolic HEAD branch changes.
- Remaining concrete layout differences include Project header controls, dedicated left Find presentation, panel relocation on layout reset and task hide/reveal edge cases, alongside the remaining prototype acceptance matrix. Goal remains in progress.

## Prototype parity completion (2026-11-02)

- Bottom tabs render tool icons, a Problems count badge and a `Local` label for the active terminal.
- The project panel gained a footer with the real visible-worktree count; Restore Default Layout validates the target dock and defers size/visibility reset until panel settings settle.
- Search rows carry per-file ZedX icons and a worktree/External source column; Actions uses the prototype's `Include disabled actions` label; the footer exposes `Open in Right Split`; `Alt+N` and `Ctrl+1–6` select scope and category.
- The desktop top bar shows the labelled `Search Everywhere` entry with the ⇧⇧ hint; compact keeps the icon button. Left and right rails use the prototype's 44px and 42px widths, including compact overlay offsets and dock-drag math.
- Added GPUI regressions for identity/main-menu/search-label rendering, the project footer, panel relocation, bottom-tab rendering, search source/icons, category shortcuts and rail widths. `integration-test` now also runs the `title_bar` tests.
- Validation: `check`, `build`, 52 zed + 2 title_bar + 6 policy + 5 asset + 5 gesture tests. Clippy passes for zed/title_bar; workspace passes with the pre-existing upstream deprecations allowed.

## Tool window menu and search shortcuts (2026-11-02)

- The left rail gained the prototype's `More tool windows` menu: `zedx_workbench::zedx_tool_window_entries` collects every enabled panel from the left, bottom and right docks, and the popover groups them under Left/Bottom/Right headings, checking the active one and toggling through each panel's own `toggle_action`. Reuses `ui::PopoverMenu`/`ContextMenu`; no upstream seam.
- Search Everywhere binds Cmd/Ctrl+Enter to the same Find in Files path as the toolbar button. The handler defers the dispatch because the workspace is already leased by the running action; `SearchWindow::find_in_files_request` builds the request so both callers share it.
- Added GPUI regressions for the menu (opens from the rail, lists a bottom-dock panel, activates Structure) and for Cmd+Enter (transfers the query, leaves Actions untouched). Both were validated by mutation: dropping the bottom dock from the listing, and removing the key-reset wiring, each fail the intended test.

## Git history tool window (2026-11-02)

- `workspace::zedx_actions::ShowGitHistory` backs a left-rail `Git history` button. It focuses the upstream `GitPanel` and switches it to the History tab, so the prototype's Git tool window reuses the existing commit log instead of a new view.
- Upstream seams: `GitPanel::show_history` is a public wrapper over the private `set_active_tab`, because `ActivateHistoryTab` is handled on a rendered node and therefore only fires when focus has already reached the panel; `showing_history_tab` is a `test-support` accessor; `git_ui` joins the `zed` dev-dependencies with `call` and `test-support` features so the regression can assert the tab.
- Remaining prototype gap: the branch sidebar (`.git-branches` tree with remote branches and its search box) has no native counterpart. The History tab lists commits only.
- Validation: `integration-test` 63 zed + 2 title_bar tests; the new test was mutation-checked by dropping the `show_history` call, which fails the "must land on the History tab" assertion. Clippy passes for `workspace` and `git_ui` libs.

## Branches tool window (2026-11-02)

- New `Branches` panel (`zedx_panels::Branches`, `workspace::zedx_actions::ToggleBranches`) puts the prototype's branch sidebar in the left dock by embedding the upstream `git_ui::branch_picker::BranchList` in embedded mode. No upstream seam: it only calls the public `create_embedded`.
- `Branches` rebuilds its list on `GitStoreEvent::{RepositoryAdded, RepositoryRemoved, ActiveRepositoryChanged, RepositoryUpdated}`, so a repository discovered after the window opened is listed. The alternative — capturing the repository once at panel creation — leaves the panel permanently empty in that case (the git status item had the same defect earlier).
- Validation: a GPUI regression asserts the list is empty without a repository, becomes populated after a fake `.git` is added and scanned, and that `ToggleBranches` opens the panel in the left dock. Mutation-checked by making the rebuild a no-op, which fails that assertion.

## Shared tool window head (2026-11-02)

- `ui::zedx::tool_window_head(id, glyph, title, cx)` renders the prototype's `.tool-window-head`: a debug-selectable 35px row with the ZedX glyph, a small title and a bottom border, leaving room for a caller-supplied meta label and hide control.
- `Structure`, `Find`, `Problems` and `Branches` use it. Problems adds its row count and both windows hide through `PanelEvent::Close`, which the dock turns into `set_open(false)` (`dock.rs:779`).
- Validation: `tool_window_heads_render_and_hide_their_panel` asserts all four heads render (`zedx-head-structure`, `zedx-head-find`, `zedx-head-branches`, `zedx-head-problems`) and that the Branches and Problems hide controls close their dock. Mutation-checked on the Branches handler only — note that an earlier attempt patched `Structure`'s identical one-line handler instead and produced a false pass, so the mutation was re-targeted.
- Also asserted that the `Branches` window appears in the `More tool windows` menu, which was previously inferred rather than tested.

## Branch tools and compact behaviour (2026-11-02)

- `Branches` now creates its embedded `BranchList` with `show_footer: true`, which is the same mode `git_picker` uses for embedded branch lists. That surfaces upstream's footer controls — create from the default branch, delete, switch — which are the prototype's branch tools. Testability limit: those buttons carry no `debug_selector`, so the regression cannot assert them directly; it asserts the list still populates and the panel still lays out.
- Added `branches_panel_becomes_an_overlay_in_compact_mode`: at 390px wide the left dock (with Branches active) must become the 235px overlay and leave the editor starting at 44px; restoring 1280px must remove the overlay. Mutation-checked by shrinking the overlay width constant, which fails the width assertion.

## One ordering source for rail and menu (2026-11-02)

- The IDEA order previously lived inline in `PanelButtons::render_zedx`. It is now `zedx_panel_priority(dock, name)` in `workspace/src/zedx_tool_window_bar.rs` (module made `pub(crate)`), and both the rail and `More tool windows` use it, so a panel cannot appear at different positions in the two lists.
- Validation honesty: `more_tools_menu_follows_the_rail_order` asserts `Run` above `Problems` in the menu. Swapping their priorities fails the test, but **removing the new menu sort does not fail it** — the dock's own insertion order already satisfies this pair in the fixture, so the sort is a consistency guarantee the fixture cannot isolate. Treat the test as a guard on the priority map, not as proof that the sort matters.

## Rail order state (2026-11-02)

- New `workspace/src/zedx_rail_order.rs`: a `RailOrder` GPUI global holding per-dock panel order (the prototype's `toolOrder`), with `rank`/`nudge` and JSON load/save. `zedx_rail_sort_key` is now the single ordering definition used by the rail, the `More tool windows` menu and reordering itself.
- Persistence is opt-in: only `workspace::zedx_rail_order::init_global(cx)` (called from `crates/zed/src/main.rs`) enables reading/writing `$CONFIG/zedx-rail-order.json`, so test binaries neither inherit a developer's saved order nor overwrite it.
- Trigger added: `workspace::zedx_actions::MoveToolWindow { name, delta }` → `Workspace::move_zedx_tool_window` → `PanelButtons::move_panel`. Pointer dragging is **not** implemented yet; this is the programmatic entry point the drag will call.
- Bug found by mutation testing: `nudge` originally materialised the starting order from the dock's internal `panel_handles()` order, which is activation-priority order and differs from what the rail displays, so a no-op move still appeared to reorder. Fixed by materialising the *visible* order through `zedx_rail_sort_key`; the regression now fails when `nudge` is a no-op.
- `crates/workspace/Cargo.toml` gains `paths.workspace = true` for the config directory; `Cargo.lock` records the new edge.
- Validation: 68 zed + 2 title_bar integration tests, 4 new `zedx_rail_order` unit tests, clippy clean on `workspace` lib.

## Rail drag reordering (2026-11-02)

- Rail icons are now drag sources and drop targets: the icon div is made stateful (`.id(..)`) and gets `on_drag(RailDrag { name })` plus `on_drag_move::<RailDrag>`. Hovering a sibling moves the dragged panel to that slot through `RailOrder::move_to`, then notifies the dock so the rail and `More tool windows` repaint.
- Because the trigger closure only receives `&mut App`, the handler works from a cloned `Entity<Dock>` instead of `Context<Self>`; that also avoided borrowing `self` across the closure (`E0521`).
- Testability limit: the pointer drag itself is not simulated — upstream drag tests call the drop handler directly, and this code does the equivalent through `RailOrder::move_to`, which has unit coverage (`move_to_targets_an_index_and_clamps`, mutation-checked). The GPUI-level guarantee is that the workbench still renders and the programmatic reorder path still passes.
- Removed a `PanelButtons::move_panel_to` helper that became dead once the drag handler went through the order directly; clippy with `-D warnings` caught it.
- Validation: 68 zed + 2 title_bar integration tests, 5 `zedx_rail_order` unit tests, clippy clean on `workspace` lib.

## Restore Default Layout resets the rail order (2026-11-02)

- The prototype's `resetLayout()` ends with `resetToolOrder()`; the native reset only restored docks and sizes, so a custom rail order survived. `RailOrder::reset` clears every dock slot (and persists), and `restore_zedx_layout` now calls it.
- Validation: `restore_default_layout_forgets_the_custom_rail_order` moves Branches to the front, checks the menu reflects it, dispatches `RestoreDefaultLayout`, and requires the default order back. Mutation-checked by dropping the `order.reset()` call, which fails the test.

## Git history in the tool window menu (2026-11-02)

- The prototype's tool menu lists tool windows *and* the Git log entry. The `More tool windows` menu now ends with a separator and a `Git history` entry that dispatches `ShowGitHistory`, so the Git log is reachable without the dedicated rail icon.
- The existing Git-history regression now drives that entry by clicking it instead of dispatching the action directly, which also proves the menu handler dispatches: mutation-checked by replacing the handler with a no-op, which fails the test.
- Remaining Git-window gaps are now precisely recorded rather than vaguely: upstream `BranchList` offers `BranchFilter` plus `ShowLocalBranches`/`ShowRemoteBranches` actions, but the embedded Branches window does not surface them, and the prototype's grouped local/remote tree, log filter chips and commit-details pane have no native counterpart.
