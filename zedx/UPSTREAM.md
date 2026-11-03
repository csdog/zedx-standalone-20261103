# Upstream integration

Base: `zed-industries/zed` **v1.26.2**, commit `c4273271092b0287f7d181f616bd5171e0a2f70c`.
Remote: `upstream` → `https://github.com/zed-industries/zed.git`.
The local branch contains fork commits on top of this upstream baseline; uncommitted work may be present.

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
| `crates/workspace/src/zedx_layout.rs` / `zedx_bottom_bar.rs` | Restore default dock sizes and bottom tool-window tabs/clear/hide | `workspace.rs`: module declarations and one optional layout-restore state field; owned workbench calls the header |
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

## Sidebar alignment · 2026-11-02

- Default left pins follow the current HTML: Project / Commit / Structure / More at the top; Git history / Run / Terminal / Problems below. Right pins begin with Notifications, followed by AI Assistant / Outline / Database. Extra panels remain in More; saved custom ordering can still pin them. Active panels retain live nonzero badges.
- More uses readable panel labels and includes the Notifications and Database actions inside the Right group. Every rail control is keyboard focusable; defaults add Cmd+1/0/7/9/4/6 and Alt+F12 (Ctrl on non-macOS) through the existing ZedX default-keymap hook, before user bindings.
- `workspace.rs` gains an optional layout-restore controller. It waits for actual dock transfers before resetting sizes, selecting Project/Terminal and restoring editor focus, avoiding the settings-file race.
- Verified: 71 ZedX native tests, including a real ProjectPanel settings-backed move, sidebar geometry at 1280×900 and 390×500, click-to-hide, More-to-Find, and Project/Commit/Structure keyboard navigation. Full check/build and native visual inspection are tracked separately.

## Git sidebar buttons and fresh-workspace startup · 2026-11-02

- `crates/zed/src/zedx_git_history.rs` is a new bottom Panel that owns an upstream GitGraph, not a second GitPanel. It preserves the Commit editor/draft and loads only on activation. Active repository addition/removal/switch rebuilds the graph; normal graph updates remain upstream.
- Commit routes through `zedx::ToggleCommit`; Git routes through `zedx::ShowGitHistory`. Both explicitly close an already-visible owning panel, independent of the upstream close_panel_on_toggle preference. The toolbar and More menu share the Commit action; Git is a real bottom-panel entry with active state and a Git tab.
- Small upstream seam: `GitPanel::show_changes` exposes its existing tab transition without requiring a rendered action-dispatch node. Remaining integration is in owned ZedX modules (panel registration, actions, toolbar/bottom-tab labels, icon mapping, default layout position).
- Fresh standalone launch exposed an activation-priority collision: Branches=5 and CollaborationPanel=5. The fixed-panel macro now reserves 100+ for fork panels; this also prevents Find/Terminal and Structure/Debug collisions after docking changes. Regression loads the actual CollabPanel with all fork panels.
- Final validation: 74 ZedX native tests, 5 asset tests, embedded-resource build, check, and strict release/all-targets/all-features Clippy for workspace/zed/git_ui/ui pass. Runtime verifies cold launch, Commit changes, mouse toggles, repeated Cmd+9, real repository graph, commit details and diff in a separate preview profile. The main user service session remains running.


## Upper-right sidebar · 2026-11-02

- New owned `workspace/src/zedx_right_sidebar.rs` keeps Notifications / AI Assistant / Outline / Database at the upper right. Core pins no longer depend on the generic native panel rail or drift left with an old Outline preference. Disabled/loading panels retain a disabled pin; notifications and database retain the confirmed prototype toasts.
- Minimal upstream seam: Workspace owns an optional pending panel activation. The controller waits for the actual native panel to transfer before opening/focusing the right dock. Same-button close is explicit, independent of upstream close_panel_on_toggle. Dock observation keeps selection/availability live.
- Existing Agent and Outline views remain intact inside an owned Assistant/Structure header and hide button in `zedx_tool_window_bar.rs`. No provider setup or AI request is part of this UI task. Native Agent content remains upstream, not a replacement mock conversation UI.
- Right core pins support pointer dragging and MoveToolWindow. Notifications is fixed first; More follows the same core order. RailOrder.move_subset_to preserves other saved right-dock panels and JSON round-trips. Restore Default Layout clears the saved order.
- Four GPUI regressions cover desktop/compact bounds, panel toggles and header hide/editor focus, asynchronous Outline transfer, disabled Agent and actual pointer reorder/reset. Final validation: 78 ZedX regressions, 7 RailOrder tests and 5 asset tests pass; embedded check/build and strict release/all-targets/all-features Clippy pass. After the final left-aligned header adjustment, the 4 right-sidebar regressions, strict workspace/zed Clippy and embedded build pass again. Native preview verifies real Rust outline navigation to line 45, Agent/Outline switching, header hide and editor focus (45:5 → 45:6), both toasts, and the final left-aligned Structure title. Desktop drag automation did not change the order; GPUI pointer dragging and serialized-order round-trip pass, so native drag/restart remains independently tracked. Main user app/service was left running.


## Independent bottom tool windows · 2026-11-02

- User correction supersedes the earlier shared Terminal/Debug/Run/Problems/Git strip. New owned `zedx_bottom_tool_window.rs` replaces `zedx_bottom_bar.rs`: only the active Terminal/Run/Git/Debug title, tool-specific controls and hide action render. Problems/Find already own a header and receive no duplicate outer bar. Internal Terminal session tabs remain intact.
- Removed the obsolete Dock.zedx_tabs helper. The workspace integration only asks for an optional active-tool header; sidebar, More and shortcuts still select the existing independent panel entities, preserving terminal sessions and task output.
- Regression checks switching Run → Problems → Git → Run: only the selected header exists, no shared tab strip renders, the Run instance survives, and Hide closes the bottom tool window. Existing Git/header-hide tests now assert the standalone heads.
- Validation: 79 ZedX regression tests pass, strict release/all-targets/all-features Clippy for workspace/zed passes, and the embedded-resource app builds. Native preview verifies separate Terminal/Run/Problems/Git headers with no shared tabs; returning to Terminal retains the original shell session. No commands were entered into the shell and no AI service was configured. The main app/service session stays running.


## Top bar proportions · 2026-11-02

- User feedback identified the 34px upstream platform row as visibly too narrow. New owned `title_bar/src/zedx_title_layout.rs` sets 51px desktop / 47px compact row height and centers native macOS traffic lights when the height changes. Minimal upstream seam: optional PlatformTitleBar content-height override, leaving window controls, drag, double-click and system tabs intact.
- Owned actions now use 34px controls and 8px desktop spacing/separators. Below 760px the row uses 30px-wide buttons and 4px gaps; search labels appear from 1100px. Settings opens the real settings action. Sign-in/user-menu text yields space on compact widths; Restricted Mode retains its warning icon, accessible label, tooltip and trust dialog.
- Project initials use the prototype-inspired green avatar, project text is bounded, and desktop project controls have a chevron. The owned branch trigger opens the actual branch picker once, without the duplicated worktree/branch labels, and preserves the branch visibility preference. Upstream UI mode retains its original branch/worktree renderer.
- GPUI title-bar tests cover 1280/1000/760/390/360px heights, non-overlapping project/actions, all Run/Debug/Settings/Theme button bounds, and adaptive search labeling. Validation: 2 title-bar tests and 79 ZedX regressions pass, along with strict release/all-targets/all-features Clippy for platform_title_bar/title_bar/workspace/zed and the embedded build. Final branch-preference guard was rechecked with title-bar tests, strict title-bar/platform Clippy and embedded build. Native preview verifies 51px desktop spacing, the real Branches/Stashes popup, Settings opening its real window, approximately 392px compact layout with all five operation icons visible and the Restricted Mode icon retained, and titlebar double-click restoring a maximized layout. Settings were only viewed; no branch switch or trust change was made. Main user app/service remains running.


## Git log branches and toolbar · 2026-11-02

- New owned `zedx_git_history_branches.rs` reads real repository branches, merges tags from the shared All graph metadata cache, groups Local/Remote/Tags, shows current markers, virtualizes rows and supports query/collapse/all/reset/refresh. Cancelled tasks and repository/generation guards reject stale branch lists. Tag scans are cached by metadata count and invalidated on ref changes; removed selected branches return to All. New/delete branch controls retain the HTML prototype notices, not repository mutations.
- New owned `git_ui/src/zedx_git_graph_toolbar.rs` is opt-in for the embedded ZedX log. It reuses the native search editor, keeps its search key context/confirm behavior, renders the Branch/User/Date/Paths hints, and opens the real column-visibility menu. `set_zedx_log_source` retains the graph/editor instance while clearing old selection, diff/message tasks and search matches. Upstream graph tabs use their original toolbar.
- Small upstream GitGraph seams: child module/export, an optional toolbar width initialized to None, and an explicit alternate toolbar call. Actual layout is owned by GitHistory and accounts for open side docks before showing the 220px branch column, branch chip and secondary hints.
- Validation: 84 ZedX regressions pass (five new Git branch/layout tests), strict release/all-targets/all-features Clippy for git_ui/zed passes, and the embedded app builds. Native isolated fixture `/private/tmp/zedx-git-log-fixture-t_ontfqa` has three disposable commits, divergent main/feature branches, a local remote-tracking ref and a tag. All → feature/ui → v1.0 displays 3 → 2 → 1 actual commits. Group search, hide/show, native column options, commit details and clearing old details on source change are verified. Keyboard resizing expands the log to show all groups. Git still reports main and a clean fixture; no project-repository commits or pushes were made.
- Remaining Git parity is explicit: text search still uses native match navigation instead of row filtering, and native details keep their larger metadata layout. Static User/Date/Paths labels match the HTML spans. These are not evidence that the complete prototype goal is achieved. Assistant content and the other unverified parity rows remain active work.

## Top-bar spacing refinement (2026-11-02)

- In response to the user’s remaining narrow-toolbar feedback, owned title modules now use a 56px desktop / 50px compact row, 40px desktop buttons, 20px desktop icons and 10px group spacing. Intermediate widths retain 34×38px controls; compact retains 30×34px controls and 4px gaps. Project avatars are 30px desktop / 26px compact. Branch and menu triggers share the control sizing.
- Run configuration now has the same explicit hit height and yields below 1000px, keeping the five primary operation buttons visible. Native recent-project and branch actions remain in place. No additional platform-title-bar seam was needed; the existing owned branch call now passes the window to select its size.
- Validation: two title-bar tests passed, with geometry checks expanded to nine widths including both responsive boundaries; strict title-bar Clippy and the embedded build passed. Native preview verified the desktop Search Everywhere click, approximately 392px compact with all five operation icons visible, and title-bar double-click restoring a maximized window. Only the isolated preview was restarted.

## Compact Git commit details (2026-11-02)

- Added owned `git_ui/src/zedx_git_graph_details.rs`. The native GitGraph seam selects it only for the ZedX history surface, and only at viewport widths of at least 1100px, as in the HTML. Upstream GitGraph keeps its existing details renderer. The owned toolbar opt-in initializes the existing split ratio to 74% / 26%; user resizing still uses the existing split state.
- Files take the upper 58%, with a 29px count / Show Diff / close header. Message and author/date/hash take the lower 42%; long Markdown scrolls independently. Native ChangedFileEntry activation and open_selected_commit_view are reused for actual commit diffs. Only messages matching the selected SHA are shown, preventing an old asynchronous message from being displayed under the next commit.
- Validation: all 31 Git-graph tests pass, including owned 202px/346px bounds, scrolling and stale-message rendering regression; strict git_ui Clippy, embedded app check/build and diff whitespace check pass. Native disposable three-commit fixture verifies visible README at the default 202px, actual Main diff, Feature message replacement, light/dark themes, compact hiding and desktop restoration, and final 26% default detail width. Fixture remains clean on main. Only the isolated preview was restarted.
- Still pending: actual log row filtering and the prototype Revert / Expand notice controls. This slice does not complete the full prototype goal.

## Live Git log row filtering (2026-11-02)

- Added owned `git_ui/src/zedx_git_graph_filter.rs`, containing editor observation, streamed query handling, stale-result checks, the virtualized filtered view and visible/original row mapping. The upstream seams add one state field, opt into owned search/rendering only for ZedX, route six navigation actions, and translate scroll indices. The original graph data is never replaced by a subset, so commit opening and repository-index lookups keep their original meaning. Existing native cells, ref chips, column fractions/visibility/resizing, row menus, commit diff and split state are reused.
- Filtering follows the HTML subject-substring behavior, trims whitespace and also accepts short or middle hash fragments. For a long hex-only query, a shorter text prefix obtains candidates and loaded subjects verify the full query, so the backend’s automatic hash-prefix interpretation does not suppress hex-text titles. Body-only matches are rejected. Query tasks are canceled/replaced and guarded by generation, repository and source. Empty results clear selection/details; clearing the query restores the native graph and cancels old results.
- Validation: 33 Git-graph and 84 ZedX regressions pass, including two new rendered-view tests for live input, mouse/keyboard original-index mapping, trimmed/short-hash/hex-text matching, body-only rejection, empty states, rapid replacement and clearing. Strict git_ui/zed Clippy, embedded app check/build and diff whitespace checks pass. Native disposable fixture verifies All/Main → one matching row, correct Main diff, feature/Main → none, feature/Feature → one, clear → two, All and ad3 middle-hash → Main, actual context menu Commit 252ad3b, dark-mode Down selecting Main among two results, compact query/selection retention and final clear restoring all three commits. The fixture stays clean on main; no real project commit/push or main app restart.
- Remaining: the filtered Graph column shows colored markers/ref chips but not DAG links. Its graph rendering and filter-field geometry need further prototype alignment, alongside Revert/Expand detail notices and long-history cancellation/scrolling runtime coverage. Full prototype goal remains active.

## Filtered Git graph and toolbar parity (2026-11-02)

- Added owned `git_ui/src/zedx_filtered_graph.rs`. It walks actual commit parents iteratively until the nearest visible ancestors, deduplicates collapsed merge ancestry and renders clipped per-row paths/circles with original branch colors and compacted lanes. Hidden-commit bridges use reduced opacity; unrelated sibling commits are never connected. Projection caching is keyed by graph length and visible indices and cleared with a new query. The only upstream seam is the module declaration; owned filtering now renders this projection instead of standalone commit glyphs.
- Owned filter toolbar now uses a 170px / minimum 80px, 24px-high bordered input and an elastic spacer before refresh/options, placing Branch/User/Date/Paths beside the input as in the HTML. The existing rendered-view test asserts field geometry. Commit details add Revert / Expand controls with the prototype’s non-mutating notices, while Show Diff remains functional.
- Validation: all 36 Git-graph + 84 ZedX regressions pass; three new model tests cover merge/hidden-ancestor projection, sibling separation, missing/cyclic data and a 10,000-node hidden chain. Final path-error logging refinement was rechecked with all three model tests, strict git_ui/zed Clippy, embedded app check/build. Native independent fixture `/private/tmp/zedx-filtered-dag-fixture-60o503p4` has six real commits with a merge and two hidden branch commits; Keep produces four rows with direct fork/merge and faded hidden bridges, both notice toasts appear, and light/dark, ~854px / ~392px compact layouts, desktop restoration and clear-to-six work. The fixture remains clean on main. Preview uses isolated `profile-filtered-dag`; the earlier three-commit fixture and main user app are untouched.
- Remaining full-goal work includes the owned Assistant body/composer and the outstanding long-list/cancellation, external-LSP and interaction evidence in PROTOTYPE_PARITY.md. This is progress, not completion of the prototype goal.

### Assistant welcome surface

- `crates/agent_ui/src/zedx_assistant.rs` owns the prototype welcome text, live active-file context, retained three-line input and no-provider send notice.
- `AgentPanel` has only the owned entity/subscription, activation/focus and render seams. The welcome surface applies to an uninitialized or empty native thread with no authenticated provider. Existing messages, native drafts, external agents and terminal sessions keep the upstream renderer. `ZEDX_UPSTREAM_UI` disables this surface.
- Model registry changes notify the panel and initialize the native conversation when a provider becomes available; an owned draft is never submitted automatically. Workspace observation refreshes the context card on active-item changes. The outer dock retains the shared Assistant heading and minus control.
- Verification: two real AgentPanel GPUI regressions plus the full 105-test AgentPanel suite pass; strict `script/clippy -p agent_ui --locked --offline` and the embedded native build pass. Native preview verifies welcome/context/composer, README → feature.txt context updates, actual send notice, hide/reopen draft retention and light/dark appearance. Existing fake Assistant dock tests cover dock controls separately.

### Commit search cancellation and long lists

- `crates/git/src/repository.rs` commit search now observes result-channel closure while waiting for output, returns immediately when delivery is cancelled, and enables kill-on-drop for its Git child. This prevents waiting for a child that is blocked writing to an unread stdout pipe.
- `crates/git/src/zedx_commit_search_tests.rs` uses a real 20,000-commit disposable repository and a bounded result channel. It waits for a real result, cancels three searches and verifies a subsequent search still works. Its wall-clock deadline explicitly measures an external process rather than virtual GPUI time. Fixture commands use `GitBinary::build_command` and asynchronous pipes.
- The owned filtered-view test fixture also supports 4,000 commits: it verifies 2,000 visible rows, virtualized first/last bounds, original-index selection on the last row, quick replacement with the opposite half and clearing without modifying graph data. All 80 Git backend and 37 Git graph tests pass, as does strict Clippy for both crates. Native 20,000-commit fixture `/private/tmp/zedx-long-history-fixture-u5h_ln71` verifies Keep End → 00001/3c0d32f, wheel scrolling and click → 00271/c638ecf, Other End → 00002/9b2efce, replacement with Needle → only 00000/ee1da42, and clearing back to the full alternating history. Fixture counts and endpoint hashes were independently checked with Git CLI. The embedded native build passes.

### Actions search availability

- `crates/zed/src/zedx_search_commands.rs` owns collection and scope semantics. It snapshots current-context commands before opening the modal, adds default-buildable registered commands as unavailable, filters hidden actions and deduplicates canonical names. Parameter-requiring commands without default arguments are omitted.
- The existing search delegate keeps stable action indices, excludes unavailable entries unless Actions explicitly includes them, marks rows/preview/footer unavailable, and guards both confirmation paths before emitting dismissal or dispatching an action. File scope and action inclusion have independent state; repeated search shortcuts toggle the current category's relevant state.
- The owned controls now follow the prototype: All/Actions checkbox only, other categories scope/type filter. Two new GPUI regressions cover actual control clicks, unavailable-row confirmation, hidden-command exclusion, available command dispatch and independent scopes. All 86 ZedX tests, strict Clippy and embedded build pass. Native Transpose verification proves default exclusion, checked disabled row, inert click/Enter and correct Files controls.
- Native standalone Shift verification remains manual: CUA explicitly rejects modifier-only key presses; this limitation does not replace the six passing GPUI gesture-event-chain regressions.


### Roomier title bar

The owned `crates/title_bar/src/zedx_title_layout.rs` keeps sizing policy out of upstream rendering: 64px desktop / 56px compact height; 44px desktop controls, 36×40px intermediate controls and 30×36px compact controls. Project identity uses a 34px / 28px avatar and wider desktop spacing. Owned title actions add light outlined Run configuration and Search Everywhere triggers with 12px desktop gaps. Native traffic lights continue to follow the content-height override.

### Fast tool-rail drops

Owned `crates/workspace/src/zedx_rail_drag.rs` handles only the visible pinned subset, checks target bounds, normalizes insertion after removing the source and preserves unrelated saved slots. Both tool-window and right rails finalize on drop using prepainted child bounds: a fast native drag can start in the bubble phase after the only drag-move capture callback has already run. Two single-move-release GPUI regressions fail without the drop fallback. All 93 ZedX regressions, strict workspace/zed checks and the embedded build pass. Native preview verifies left/right reorder, outside drop rejection, restart persistence and clearing saved order with Restore Default Layout.


### Fast dock-resize release

`zedx_workbench.rs` finalizes a `DraggedDock` drop using the same absolute pointer-to-dock coordinates as the upstream continuous drag handler. The finalizer lives on the owned workbench, honors zoomed layout, updates the existing dock size state and serializes the workspace. No additional upstream render seam is needed. Three real GPUI pointer regressions cover left/right/bottom one-move release: each failed with the unchanged 300px size before the fix and applies approximately 350px after it. Existing continuous mouse drag and keyboard/reset tests remain in the full 96-test ZedX regression suite.

Native independent `ZedX Resize Preview` confirms left 265→328px, right 300→363px and bottom 202→265px with one-move drags, followed by actual restart restoration of workspace ID 10 and its three saved size records. Double-clicking each gap returns to 265/300/202px. For project-restore verification, launch the project directory only: including an explicit file alongside the directory constructs a fresh workspace record, so that test does not exercise reopening the saved workspace. Embedded build/check and local installation pass; the main user app was not restarted.


### Prototype title-bar alignment (supersedes the roomier trial)

The user requested alignment with `prototype/index.html`, including dimensions and button order. Replacement owned modules `zedx_prototype_project_identity.rs` and `zedx_prototype_title_actions.rs` replace the earlier identity/actions files through path-module seams. The current CSS is 51px desktop / 47px at ≤760px; right buttons are 32×34px with 20px icons, 7px / 4px group gaps, and 18px separators. Search is icon-only, Settings opens Search Actions, and theme remains available there. Project avatar/name/chevron share one recent-project trigger. The prototype has no extra main-menu/theme/sign-in controls; ZedX hides those title-bar surfaces while the upstream fallback keeps them. The real run configuration observes task inventory and displays the last scheduled task. Twelve-width geometry tests now use a long project name, include the 760/761 and 1100/1101 boundaries, and assert button order, dimensions, alignment and 23px avatar size.

### Symbol-search failure reporting

`LspStore::symbols_with_errors` and its small `Project` wrapper opt into failure reporting only when no server supplies results. The existing `symbols` API preserves its behavior; successful results survive another server’s failure. Both use the same request/conversion implementation. ZedX search opts into the reporting API. Owned `zedx_search_lsp_tests.rs` exercises the LSP protocol for external results, hidden-worktree exclusion after opening, Classes filtering, split navigation, real errors/recovery, compact footer bounds and partial server failure. Before the fix, the error test fails with `None`; afterwards all three focused tests and the 99-test ZedX suite pass. Error status is bounded/truncated with a full-message tooltip. Native protocol-fixture verification remains pending because title-bar alignment was prioritized by the user.

The user subsequently found the literal HTML height too tall in the native preview. The final native override is 40px desktop / 38px compact, retaining the prototype’s 32×34px controls, button order, avatar and spacing. This supersedes the intermediate 51/47px container height above.

Final 40/38px title-bar verification: two title-bar tests pass across twelve widths, strict title_bar checking and the embedded build pass. Native independent preview verifies both heights (desktop and ~392px compact), avatar → recent-project popover and gear → Actions. Full ZedX regression immediately before the height-only refinement: 99 passed. Installed locally with backup `/Applications/ZedX.previous-20261102-142907.app`; main app remains running its older process until reopened.


### Search popup dragging and left-rail selection

Owned `zedx_search_drag.rs` stores per-modal placement, observes actual painted bounds and captures window mouse-move/up events while the header’s free area is held. Capture tracking is required because ordinary Div mouse-move listeners stop at their hitbox; the first boundary test exposed this. Positions clamp against actual popup dimensions and refit after resize/content changes. The owned search layer is deferred above compact dock overlays. Category/input/results interactions keep their handlers. Two pointer regressions and the full 102-test suite verify movement, retained focus/modal/input, close and viewport clamping.

The tool rail previously used `toggle_state` without an explicit selected style, so its normal transparent style did not paint any selection. The owned tool-window renderer now adds a 32×34px, 6px-radius selection surface in prototype colors and uses a tinted SVG mask for the selected monochrome icon. Inactive icon assets remain unchanged. A real Project/Commit/Problems pointer regression covers active/open state, focus independence, separate upper/bottom groups and same-pin close. Native light appearance verifies Commit and Terminal highlights independently and the selected SVG tint. An independent fixture verifies a 200×160 physical-pixel search-header drag followed by typing README: position, focus and results remain correct. Final native dark/compact layering verification remains. Strict workspace/zed checks, embedded build/check and local installation pass; backup `/Applications/ZedX.previous-20261102-143852.app` retained. Main user app was not restarted.


### Readable native typography

The user found operation/title text too small. Owned `ui::zedx::label_size` and `caption_size` now express primary UI text at the configured font size (1rem) and metadata at 12/13 of that size, replacing the extra Small/XSmall multipliers in owned surfaces. The shipped UI font default changes from 13 to 14 in one `assets/settings/default.json` hunk; explicit user preferences still take precedence. No editor font default is changed. Tool-window/project titles gain medium weight and shared header glyphs grow from 14 to 16px. The 40/38px top-bar height stays fixed. Project trigger content is width-bounded on compact windows, only text truncates, and the avatar remains 23px. Twelve-width testing caught both a clipped settings button and an avatar shrinking to 17px; the width limit, non-shrinking avatar and truncated label resolve both.

Typography validation: final 102 ZedX regressions, two title-bar tests at twelve widths, one compact Git-details test and two Assistant tests pass. Strict Clippy across all six affected packages, embedded build and final check pass. The dedicated Typography Preview uses the shipped default with no UI-size override; native project/header/run text and Search categories/results/footer were inspected at desktop width. Installed with backup `/Applications/ZedX.previous-20261102-145444.app`; main working app was not restarted. The separate compact modal-layering native audit remains open.

### Sidebar material

ZedX Light/Dark now use a transparent `panel.background` content token. The dock paints the sidebar material once from the solid theme surface at 62% opacity; ordinary tree rows, titles and Assistant contents no longer layer another opaque panel background over it. The workspace root omits its extra frame fill for these transparent-panel themes, and the owned workbench frame uses 30% of its original tint. Both changes are conditional on the transparent panel token; other themes and upstream mode retain their existing background behavior. The bottom tool window and compact overlay docks keep solid surface backing to prevent editor text showing through overlays. Editor, menus and search retain their existing solid surfaces. Native material uses the existing GPUI blurred-window backend, with no OS or GPUI changes. Transparent side docks use their own rounded background instead of opaque corner paint, avoiding bright corner patches. Final 102 ZedX tests and strict workspace/zed Clippy pass. Native light/dark Project and Assistant readability and selected states were inspected; final native light preview confirms the left sidebar no longer has corner color patches. Embedded build and local installation pass; backup `/Applications/ZedX.previous-20261102-150105.app` retained. The dedicated Sidebar Preview is open; main working app was not restarted.

### Commit tabs and per-file differences

Owned `git_ui/src/zedx_commit_panel.rs` replaces the Changes header/list shell in ZedX mode. It provides Commit / Stash / Shelf tabs with live counts, a 35px underline header, 30px toolbar and collapsible Changes / Unversioned Files groups with left-side staging checkboxes, status, filename and trailing directory. It reuses the real GitPanel repository/status/staging/commit editor and operations; the existing composer and commit options remain upstream, so exact prototype composer alignment is still open. Stash displays actual cached repository stashes and applies the selected stash by resolving its OID against current data. Shelf explicitly reports unavailable; no synthetic shelf records or Git-stash-as-shelf conversion are introduced.

File clicks and panel Confirm now call SoloDiffView for exactly the targeted GitStatusEntry, avoiding ProjectDiff and ignoring other marked files. Reopening the same path reuses its tab. Explicit all-project diff actions remain available through their existing actions; upstream UI mode retains original rendering and primary-click behavior. New pointer/Confirm regression checks one target, two independent file tabs, reuse and no ProjectDiff; tab switching checks draft retention and header/toolbar geometry. Broader GitPanel regression found a pre-existing test that navigated right while the fork defaults Commit to the left; its fixture now explicitly docks right to test that navigation. Validation: 81 GitPanel regressions and 102 ZedX regressions pass; the final two focused tests additionally verify checkbox staging without opening a diff. Strict git_ui/zed Clippy and embedded build pass. Native independent fixture shows Commit 2 / Stash 1 / Shelf 0, two changed files plus one unversioned file, and the real saved stash. Clicking main.rs then README opens independent single-file differences with their own tab/path; no combined changes view is opened. Installed locally with backup `/Applications/ZedX.previous-20261102-151407.app`; main working app was not restarted.

### Source / single-file diff shortcut

Owned `git_ui/src/zedx_source_diff.rs`, nested under SoloDiffView, registers `zedx::ToggleSourceDiff`. Its exported binding helper is called after the default keymap through the existing owned panel-key setup. Control+D is restricted to `Workspace > Pane > Editor && mode == full`; commit messages, search input and terminals keep their existing shortcuts. Upstream UI mode does not register or bind the action. Small seams are the module declaration, Git UI initialization/export and the owned key-binding call.

Diff → source opens/reuses a real Editor for the working buffer, using the right-hand diff cursor, and never creates a source for deleted files. Source → diff reuses the SoloDiffView for the same buffer or opens exactly that file using the existing Git status resolver. New diff creation is deferred until the workspace action finishes, avoiding a workspace entity re-entry panic caught by the keyboard regression. Text anchors restore the cursor where visible excerpts allow; locations outside excerpts retain the diff's existing cursor. Clean files with no existing diff show a no-changes notice instead of accidentally executing Delete. Two real-key regressions cover three round trips, text/cursor/tab retention, another file, a clean file, and normal Control+D deletion inside the commit message. All 83 GitPanel and 102 ZedX regressions and strict git_ui/zed checks pass; embedded build passes. Native independent preview confirms README source → diff → source → same diff via actual Control+D, with two tabs total, unchanged text and restored 3:23 cursor. Installed locally with backup `/Applications/ZedX.previous-20261102-152321.app`; main working app was not restarted.

### Commit message resize

Owned `git_ui/src/zedx_commit_resize.rs` adds a 5px separator above the existing commit footer. Pointer handling uses window capture listeners, including final mouse-up coordinates, so one-move drags and release outside the original hitbox still work. The separator has its own focus handle for Up/Down (16px), Home and double-click reset. Default height follows 38% of the panel, with a 170px message minimum where space allows and 90px reserved for file rows plus header/toolbar/handle. Short windows clamp the visible height without discarding the requested desktop height.

The message uses the existing Editor in Full mode with internal scrolling; two small footer sizing seams let it fill the resized region, while commit controls remain fixed and visible. Draft and requested height stay in the same panel instance across tab changes and sidebar close/reopen; height is session state, matching the HTML's in-memory `commitMessageHeight`, not restart persistence. ActivateChanges now changes only the tab rather than resetting all owned state. Two real-input regressions cover a one-move 80px resize, outside release, upper/lower limits, compact/desktop changes, footer button bounds, retained draft, keyboard focus and double-click reset. All 85 GitPanel and 102 ZedX regressions, strict git_ui/zed Clippy and embedded build pass. Native independent preview verifies a 160-physical-pixel upward drag (message grows), retained typed draft, visible action buttons and double-click restoration to the original height. Installed with backup `/Applications/ZedX.previous-20261102-153600.app`; main working app was not restarted.

### Window outer corners

The user clarified that the issue is the two outside corners of the top title bar. The owned title layout now requests the prototype's 13px outer radius through a small GPUI window/platform method. Its default platform implementation is a no-op; the owned macOS helper `gpui_macos/src/zedx_window_corners.rs` uses public NSView/CALayer APIs to clip the common native frame containing the window outline, blur material and Metal content. It avoids separate painted corner patches or a title-fill-only radius, which leave the native outline outside the requested curve. Fullscreen requests zero radius. Only ZedX title layout invokes the method; classic UI and other window surfaces retain their existing decorations. Public API, platform override and module declarations are the upstream seams; native clipping stays in the owned file. Title-bar tests pass across twelve widths and all 102 ZedX regressions pass. Final strict checks for gpui_macos/title_bar/zed and embedded build pass. Native independent Corners Preview verifies the two outer title-bar corners, fullscreen square edges and radius restoration after exiting fullscreen, with native controls still present. The existing user-operated Message Resize Preview was preserved. Installed with backup `/Applications/ZedX.previous-20261102-155501.app`; main working app was not restarted.


### Right-rail selected state

The owned `workspace/src/zedx_right_sidebar.rs` now paints the same 32×34px, 6px-radius selection surface and tinted SVG used on the left rail. Light/dark palettes match the left controls. Selection follows the active, open right-dock panel rather than keyboard focus; Assistant and Outline are exclusive, and closing the dock clears the highlight. Notifications and Database remain their existing notice actions without a persistent panel selection. Button actions, availability and rail dragging are retained. The added real-pointer regression checks both panels, highlight bounds, editor focus, same-pin close and header hide. All six right-sidebar tests and 103 ZedX regressions pass.

Strict workspace/zed checks, embedded build and diff whitespace checks pass. Installed locally with signature verification and backup `/Applications/ZedX.previous-20261102-160236.app`. Main working app and user-operated previews were not restarted; this change has rendered GPUI pointer/geometry coverage, without a new native visual preview.


### Direct Settings entry points

The latest user request changes the gear from action search to the existing graphical Settings page via `zed_actions::OpenSettings`. Only the owned title-actions renderer changes. A small post-processing seam in `zed/app_menus.rs` moves the existing Open Settings action immediately before its Settings submenu in the application menu, without duplicating it or changing project/file/keymap/theme entries. Classic `ZEDX_UPSTREAM_UI` retains upstream menu nesting. The title-bar pointer test checks the real button dispatch; a menu regression checks direct placement, no nested duplicate and retained project settings.

Both title-bar tests (including twelve-width layout and the gear pointer dispatch), the menu regression, strict title_bar/zed checks, embedded build and diff whitespace checks pass. Installed with signature verification; backup `/Applications/ZedX.previous-20261102-160614.app` retained. Main app and previews were not restarted.


### Project / Structure stacked split

The requested lower-left Structure target is an owned extension to the single-active-panel upstream dock. `workspace/src/zedx_split.rs` owns split state, rendering, the shared rail/header drag payload, lower-half target preview, 5px ratio separator, independent close behavior and active-state synchronization. Existing Project/Structure entities are reused. Dock retains its primary active panel for existing focus/actions and renders both only when the split is enabled and either member is active. Other tools occupy the full dock; returning to a member restores the split. Narrow-window sizing clamps each section, fast single-move release is accepted, invalid/outside/other-panel drops are inert, and the default-layout action clears the split. The split preference and ratio are session-local; no new workspace database schema or on-disk layout setting is introduced.

Small upstream seams add Dock state/default, active/open/close/remove lifecycle synchronization and the owned renderer, plus per-panel workspace close handling. The owned rail renderer highlights both visible panels and closes only the chosen member. Structure's owned header shares the rail drag payload; its existing header selector remains intact. Three real GPUI pointer regressions verify rail and header drops, geometry, selected pins, editor focus, both header hide controls, resize, compact/desktop round trip, reset, rejected drops, tool switching and a pin close. All 107 ZedX regressions and strict workspace/zed checks pass.

Embedded build and local installation pass, with signature verification and backup `/Applications/ZedX.previous-20261102-163405.app`. The independent Split Preview remains in Restricted Mode and natively verifies dragging the Structure header into the lower half, Project above the live Markdown outline, and both pins highlighted. The user then opened Assistant in this preview; its state was preserved. Native rail/resize checks were not completed, so those behaviors rely on the rendered GPUI input regressions. The new-app CUA launch timed out after a long wait even though the app started; binding its already-running bundle ID then succeeded immediately. No main app restart, commit or push.

Final focused regressions also pass with intermediate-frame continuous separator dragging in addition to single-move release. Final strict zed checks pass after that test extension.


### Threads sidebar footer cleanup

The user identified the far-left sidebar toggle / history / recent-project buttons. ZedX omits that entire footer row, moves the sidebar toggle to the bottom of the tool rail and places Thread history in More. The existing project identity remains the project-opening entry point. `workspace/src/zedx_navigation.rs` owns the new control and history action handler. The toggle follows side/open/enabled state and uses the rail selection palette. The history entry is hidden when the sidebar feature is disabled.

A small default `Sidebar::show_thread_history` method and its type-erased handle delegate provide direct access to the existing archive, without relying on a hidden sidebar’s focus dispatch path. The owned action defers opening the parent, and the handle defers sidebar mutation so the archive can safely read the parent workspace. Reopening focuses the existing archive filter and preserves its view. The Sidebar renderer omits its footer only in ZedX mode; classic UI is unchanged. A real Sidebar/MultiWorkspace pointer regression covers footer removal, the new pin opening/closing, More → history from the closed state, and repeated history activation.

Validation: the real Sidebar/MultiWorkspace pointer regression and all 107 ZedX regressions pass; strict workspace/sidebar/zed checks and diff whitespace checks pass. The relocated toggle retains the original sidebar-position context menu. During this task the user added local development rules: build only the checkout’s development application unless installation is explicitly requested. Installed applications and existing user-operated previews remain untouched.

Build environment note: raw `rustup run ... cargo` picked Homebrew rustc 1.99.1 from PATH. Use the project script or set PATH/RUSTC explicitly for 1.98.1. The original debug dependency directory also stalled in kernel directory reads; an isolated target at `/private/tmp/zedx-navigation-target` reuses cached libraries and fingerprints without enumerating that directory. The final 1.98.1 regressions use that target. No cache contents were deleted, and no source rules were changed by this task.

Final embedded build passes. Updated only `/Users/cscs/IdeaProjects/zedx/target/debug/ZedX.app` using an atomic development-binary replacement and the checkout bundler; its executable hash matches the isolated build. No native window was restarted and no application was installed.


### Prototype commit message composer

`git_ui/src/zedx_commit_composer.rs` owns the requested composer replacement: a 30px Amend / last commit / history row, inset bordered message Editor with the existing configured monospace font and prototype 1.6 line spacing, inline validation and independent 28px Commit / Commit and Push… controls plus options. The previous repository footer, editor-side expand/fill controls and split submit button remain in the upstream renderer. The normal owned Commit view now renders the new composer. Existing resize state and drafts are reused; collapsed height is 80px so actions remain visible.

The options popover stays open while changing current-diagnostic checking, real sign-off and the primary push preference. Diagnostic checking examines current errors only in included files; it does not promise a new analysis/build invocation. Buttons and keyboard submission require included files (or an amend), a nonblank explicit message and existing commit access/conflict/busy checks. Inline warnings retain the configured title-length limit. The editor is temporarily read-only during an owned commit, and the completion hook unlocks it before upstream success cleanup. Successful push requests then use the existing push/remote-selection path, guarded against repository or branch changes; failed commits preserve drafts and never offer a push. Normal upstream commit behavior is otherwise retained.

`zedx_commit_message_history.rs` owns a focused, dismissible, bounded popover subscribed to repository graph/detail updates. It requests at most twenty recent commits and uses the real full message when selected, with loading/error/empty states and a repository guard. Amend reads the already-loaded HEAD details synchronously, avoiding an asynchronous fetch overwriting later edits; the existing per-repository original-draft restoration remains.

The upstream GitPanel file has only four added lines: completion callback and test inclusion. The owned state/module/render seams are in `zedx_commit_panel.rs`. Six new rendered GPUI tests cover 265/180px and compact widths, control order and bounds, mouse and keyboard validation, selected-only real FakeFs commits, full-message history, amend draft restoration, persistent options, current diagnostic rejection and bypass, sign-off, success-before-remote-choice and failed-commit retention. Tests use disposable FakeFs repositories; no task-repository commit or push occurs. All 91 GitPanel regressions, 19 owned Git UI tests, 107 ZedX regressions, strict git_ui/zed checks, embedded build and whitespace checks pass.

Only the checkout development bundle `target/debug/ZedX.app` is updated, using the isolated target documented above and an atomic binary replacement. Installed applications and existing user-operated previews are preserved. No new native preview was launched; visual evidence in this task is rendered GPUI geometry and real-input coverage.

### Sidebar material correction

`zedx_workbench.rs` paints frame tint on rails, gaps, top/bottom strips and the center column, leaving side dock areas transparent down to GPUI's native blurred window. Do not restore a full workbench background beneath these docks: the two alpha layers compound and obscure the backdrop. `dock.rs` owns the single side surface tint (90% light / 75% dark); compact overlays and bottom tools keep opaque backing. Theme frame colors follow the prototype rather than multiplying the light title-bar alpha by 0.30.

### Bottom Find view and popup ordering

`zedx_find_panel.rs` keeps search/state/navigation; `zedx_find_view.rs` owns the prototype bottom window rendering. Find's fixed default is Bottom/202px; `zedx_layout.rs` uses the same default. The existing real-file search and selection remain unchanged. Tests and search-to-Find handoff now expect the bottom dock.

`PopoverMenu::menu_priority` is a small generic upstream seam with the original default priority 1. The ZedX search window uses 2, and its file-filter menu opts into 3 so the menu paints above its parent. Keep the painted-order regression: hit testing alone did not detect the menu being drawn behind the search surface.

`ui::zedx::material_corners` paints only the frame portions outside a side dock's rounded shape while retaining the frame alpha. Opaque editor/bottom islands continue to use `island_corners`, whose opaque coverage prevents child text/background from leaking through their corners.

### Stash toolbar, selection and preview

`zedx_commit_stash.rs` owns the extracted Stash view and operation state. The parent commit module routes Enter, synchronizes repository selection, and retains Commit / Stash / Shelf state. `GitPanel::select_next`, `select_previous` and `activation_focus_handle` have small ZedX list-focus seams, preserving the upstream path for the classic UI and other tabs. Row clicks explicitly focus the panel before keyboard navigation.

Drop captures the selected OID and message before confirmation, rechecks the active repository/write access, and resolves the current cached index before invoking the existing repository operation. Busy state prevents duplicate requests. Completion refreshes the view even on cancellation/failure so removed records do not remain on screen; selection in a different repository is not cleared by an older operation. Enter/double-click reuse the existing CommitView, rather than applying or popping the stash.

`zedx_commit_stash_tests.rs` covers UI operations through actual repository jobs; the test-only FakeGitRepository now implements stash removal/reindexing and an explicit failure injection. `zedx_stash_operation_tests.rs` tests the real Git backend with three disposable stashes, unchanged working files after deletion, Apply preserving the remaining stash, and invalid-index failure.

### Open Local Folder destination confirmation

`workspace/src/zedx_project_open.rs` owns the post-selection confirmation and all three behaviors. The native directory selector is retained. Small seams in the global `prompt_for_open_path_and_open` and the Recent Projects `open_local_project` route selected paths through this flow only for ZedX. The sidebar's existing Open action reaches the global seam. Classic UI behavior is unchanged.

Attach uses the original workspace's `open_paths` with visible roots. New Window uses explicit `OpenMode::NewWindow` / `WorkspaceMatching::None`. This Window asks every workspace in the original project group for close consent, loads the replacement, then removes the old group's workspaces after success; other project groups remain. Reopening the same root preserves dirty items without prompting to save. Cancel changes nothing. Local folders cannot be attached to a remote project, so that option is omitted there. File-only selections bypass the project confirmation.

Six GPUI regressions exercise the actual prompt and filesystem/workspace paths: Attach, New Window, This Window, Cancel, cancelled save and repeated opening of a dirty current project. The 37-test Recent Projects suite also verifies the confirmation in its existing current/new-window entry tests.

### Project supplemental groups

`project_panel/src/zedx_project_groups.rs` owns the two prototype tree groups, flattened auxiliary rows, keyboard routing, invisible external sources and profile-local scratch files. Append these rows to the existing uniform list instead of pinning a second tree to the panel footer. Upstream visible-entry data and Git/diagnostic traversal stay unchanged; indent decoration receives neutral entries for auxiliary rows. Library content comes from actual invisible worktree snapshots, excluding normal project roots and the scratch data directory. Keep the hidden scratch worktree retained in panel state so its files survive a buffer close. Normal project selection clears auxiliary selection before action routing, protecting standard tree operations. Classic UI disables the additional rows.

Three GPUI regressions are queued in `zedx_project_groups_tests.rs`; they are intentionally not run per feature. Batch compilation and native verification are still required before claiming this slice verified.

### Ctrl+E Recent Files

`zed/src/zedx_recent_files.rs` owns a separate history-only Picker delegate and compact bare modal. It reuses workspace recent navigation history, the existing file-row/icon builder and fuzzy highlight policy, and normal file-open/adjacent-pane behavior. `zedx_search.rs` only declares and initializes the module and registers its keys; no Picker or Workspace implementation is replaced. Repeated Ctrl+E advances the same popup. Additional tests live in `zedx_recent_files_tests.rs` and await the next batch verification.

### Project/Structure split persistence

`workspace/src/zedx_split_persistence.rs` stores enabled/fraction in the `zedx_project_structure_split` KVP namespace, using the workspace database ID or session ID. The small Workspace::add_panel seam passes the already-borrowed workspace into the dock loader, avoiding a recursive entity read. Late panel registration synchronizes the retained preference once both panels exist. Persistence writes are ordered so an earlier drag cannot overwrite a later reset; disabled split/fraction reset are saved too. The existing split module saves successful drops, outside release, member close and default reset. Three tests run in the full 298-test Workspace suite.

The Project tree's measured row-height helper includes auxiliary rows; sticky ancestors retain space for a file row in short dock splits. Existing scroll, folded selection and OS-file-drag tests retain their behavior assertions and now count the additional rendered rows correctly. All 146 ProjectPanel tests and 109 ZedX tests pass for the combined batch; Ctrl+E code was added afterward and is not included in that result.


### Workbench completion source batch (execution deferred by user)

Substantial implementations stay in owned files: `workspace/zedx_notification_center`, `sqlez/inspect`, `zed/zedx_database`, `git/zedx_shelf`, `git/zedx_log_filter`, `project/zedx_repository_tools`, `git_ui/zedx_commit_shelf`, `git_ui/zedx_git_log_filters`, `git_ui/zedx_git_revert`, `zed/zedx_git_branch_operations`, and `project_panel/zedx_library_discovery`.

Upstream seams: notification construction archives the real view; Notifications stores center state; sqlez exposes a strict read-only opener; GitRepository adds default unsupported capability methods and RealGitRepository forwards them; Repository adds local job wrappers; GitGraph retains one revert-busy flag, initializes metadata filter state and refreshes filters when its graph invalidates; ProjectPanel's owned supplemental group module retains discovered roots. The SQLite browser uses the existing sqlez/libsqlite3 dependency through db. serde_json moves from Git dev dependencies to normal dependencies; ProjectPanel adds the existing workspace toml dependency. No upstream editor, picker, Git RPC schema, or GPUI implementation is replaced.

All test additions in this batch are source-only. No tests, compiler, Clippy, build, native verification, bundle update, installation or restart was performed for this batch, per the latest user request. See PROTOTYPE_PARITY.md for scope and pending verification.


### Merge revert, Shelf keys and layout flush · 2026-11-03

`git/src/zedx_revert.rs` owns mainline validation and local revert execution. The existing default `GitRepository::revert_commit` remains compatible; a new default unsupported mainline method forwards ordinary commits to it. `project/zedx_repository_tools` forwards the chosen parent; the owned revert UI displays actual parent hashes. No Git RPC messages are changed.

Shelf navigation/selection stays in `zedx_commit_shelf.rs`. GitPanel adds small first/last/next/previous, action-handler and key-context seams; the Git UI initializer binds Shelf-only delete keys. Cancellation no longer schedules a Shelf reload. Tests seed UI data through test-only methods and do not change the fake backend's capabilities.

`zedx_split_persistence.rs` adds an awaitable flush barrier. Workspace's existing `flush_serialization` gathers these tasks alongside normal serialization, covering the existing close/quit entry points. The barrier stays chained for subsequent writes. All new regression cases remain unexecuted by explicit user instruction.


### Database sessions · 2026-11-03 (source only)

`zed/zedx_database_session.rs` owns bounded connection MRU, successful query history, project-root scope, serialization, restoration, saved-connection/history menus and the application-quit flush barrier. The owned database panel subscribes to editor changes, restores its draft before observing edits, remembers schema/query successes and clears selected state on disconnect. Saved SQL is not automatically executed. No database file is written or removed by connection management. The existing `zedx_panels::init` has one initialization call for the quit hook; upstream Workspace, Editor and GPUI files are unchanged by this batch. Regression cases are in `zedx_database_session_tests.rs` and remain unexecuted per user request.


### Context menu appearance/customization

`ui/components/zedx_context_menu.rs` owns scoped catalogs, runtime rules, action de-duplication/availability guards, ordering, separator cleanup and the menu surface/row adapters. ContextMenu has one optional scope and small constructor/rebuild/render hooks. ListItem gains an opt-in menu style so other list widgets retain their appearance. The editor, Project, Terminal and Git context builders each add one scope call; the shared commit menu covers both Git log and history views.

`settings_content/zedx_context_menus.rs` owns serialized settings/schema/merge types. `zed/zedx_context_menus.rs` owns the SettingsStore bridge, customization modal and explicit completion-aware settings save. The existing ZedX search initializer calls its init so normal app and test workspaces share the registration path. No GPUI, keymap engine, original command handler or menu navigation implementation is replaced. New substantive logic stays in owned files. All new tests remain unexecuted under the user's current instruction.


### Settings-hosted menu editor

`settings_ui/src/zedx_context_menu_settings.rs` adds a standard Context Menus subpage and four-scope view cache. `page_data.rs` adds one user-scoped subpage link to Keymap; `settings_ui.rs` declares and exports the editor-factory registration API. The existing app-owned menu editor registers an AnyView factory and renders inline, so it does not need to move application-specific settings/action code into the settings_ui crate. Save keeps the page alive; Revert restores the saved draft. UI ContextMenu no longer appends a customization command. Old action bindings redirect to user Settings. All revised tests remain source-only.


### Editor gutter Run icon

`editor/src/zedx_run_indicator.rs` owns state/theme appearance selection; `assets/icons/zedx_run.svg` supplies a larger filled mask through one IconName variant. The existing runnable renderer has only the appearance lookup and selected-color hook; its click handlers remain unchanged. Test sources cover both themes, run/result states, upstream fallback and the local SVG shape. No tests or build were run.

### Context menu icons

- Keep icon resolution and default command symbols in `ui/components/zedx_menu_icons.rs`, and the Settings picker in `zed/zedx_context_menu_icons.rs`.
- Per-scope `icons` maps stable menu item IDs to packaged snake_case icon names; an empty name hides a symbol, and removing an override restores the original/default icon. Apply symbols after custom command IDs are assigned and before ordering; preserve handlers, disabled state and upstream escape behavior.
- Development build passed using `CARGO_TARGET_DIR=/private/tmp/zedx-navigation-target`; the checkout build directory stalled during compilation. Updated the checkout-local application from the completed binary. Settings previews, picker, hiding and reverting were checked manually; automated tests were written but not run.

### Scratch file creation from context menus

The Scratches tree no longer contains a synthetic creation row. `zedx_scratch_menu.rs` owns the New File menu for the scratch roots and their files/folders, reusing the existing scratch creation path. The project panel keyboard context-menu action uses the same entry. Remote projects and pending creation disable it. Build passed; regression cases were added but not run.

### IDEA feedback batch: context execution and workbench details

- Gutter Run keeps the 16px green symbol with an outline SVG. `editor/zedx_context_run.rs` and `zedx_context_run_policy.rs` build context Run/Debug from runnable ranges in the primary selection's buffer. Enclosing targets and multiple selected targets are supported without a nearest-unrelated-task fallback. Debug reuses the existing debugger scenario resolver. Captured buffer versions are checked before launch and after asynchronous resolution.
- The title Run Configuration dropdown provides Choose Configuration, Edit Configurations and Edit Debug Configurations using existing project task/debug editing actions.
- Commit checkboxes opt into `Checkbox::solid_selected`: opaque accent fill with white check/dash. Group state includes partially staged files.
- Project sticky rows use an opaque theme surface. Explicit `panel.overlay_background`/`panel.overlay_hover` tokens prevent the schema fallback from turning transparent black into opaque black.
- ZedX omits the Threads status/rail toggle and duplicate project/search/file status entries; remaining diagnostics, language-service, blame/conflict and activity items join the right side. The upstream UI escape path keeps its existing layout and imports.
- `ui/components/zedx_menu_outline.rs` normalizes primary menu symbols to the leading position and resolves 27 packaged outline assets in `assets/icons/zedx_menu`. Settings previews use the same packaged resources. External SVG APIs accept file paths, so SVG source text must not be passed to them.

Validation: development build passed using `/private/tmp/zedx-navigation-target` and the existing same-version WebRTC cache through `LK_CUSTOM_WEBRTC`. The checkout-local development application was updated and opened. Native checks confirmed outline menu symbols, disabled Run/Debug for Markdown, the editable Run Configuration dropdown, the light sticky Project root and the right-aligned status bar. Run/Debug commands were not executed. Regression cases for selection boundaries, menu icon placement/resources, checkbox fill/partial staging and sticky overlay colors were added but not run.

### Scratch file operations

`zedx_scratch_menu.rs` resolves auxiliary Scratch rows to entries in the retained hidden worktree. Its scoped menu uses native Cut, Copy, Duplicate, Paste, Rename, path copying, Finder/terminal and confirmed deletion actions. Scratch roots expose only New File and Paste. `zedx_project_groups.rs` preserves native selection for Scratch files, maps asynchronous selection changes back to auxiliary rows, and renders the existing filename editor during rename. This keeps project file operations and collision handling shared with upstream.

Validation: development build and whitespace checks passed. Two regression cases cover hidden-worktree rename and copy/cut/paste; written but not run as requested. An isolated native preview verified leading outline icons, menu contents, stem selection, confirmed rename, and same-directory copy/paste collision naming. The checkout development bundle was updated without restarting the user's active instance. The temporary preview was closed and removed.

### Light tool-window surfaces and rail selection

The light theme now gives panels and sticky overlays an opaque white background; the outer frame retains its existing translucent material. Both tool-window rails use the shared `ui::zedx::tool_window_selected_background` and `tool_window_selected_icon` presentation helpers: #3574F0 with a white SVG mask. Previously the two rails independently used a pale blue background and blue-gray icons. Keep tool-window selection separate from tree-row selection colors.

Validation: development build and diff whitespace check passed. Theme regression assertions were updated but not run. An isolated native preview confirmed the Project surface is white and Project/Database rail selections both show the shared blue background with white line icons. The preview was closed and its temporary bundle/data removed. The checkout dev bundle is updated; the user's active app was not restarted.

### Context-menu Run icon

Editor Run entries and multi-target Run submenus now use the toolbar's green (#208A3C light / #57965C dark). The packaged menu outline uses the exact path from `icons/zedx/play-light.svg`, retaining normal menu disabled treatment. ZedX submenu icons use the same outline asset resolver as leaf entries. Regression cases cover shared triangle geometry and preservation of the semantic green through outline normalization; written but not run. Development build and whitespace check passed; checkout dev bundle updated without restarting the active app.

### Main-toolbar Stop for running tasks

`title_bar/src/zedx_run_controls.rs` owns discovery, observation and termination of task terminals in both terminal-panel and center panes. The title bar stores subscriptions and refreshes membership as panels/panes/tasks change. Ordinary shell terminals and completed/unknown tasks are excluded. One running task stops directly; concurrent tasks open a selector rebuilt from current status. Each handler holds a weak terminal and delegates to `Terminal::kill_active_task`, which rechecks status and terminates the task's process group/shell. Red square Stop appears beside Run/Debug only while tasks run; output is retained. Upstream seams are a field, render observation hook, PanelAdded invalidation, and a toolbar child. title_bar depends on existing terminal/terminal_view workspace crates; no dependency versions changed.

Validation: development build and diff whitespace check passed. Status/multiplicity unit cases and an idle rendered-control assertion were written but not run. Native isolated fixture verified idle absence, running appearance, single-task termination and disappearance, two-task chooser, stopping A while B remains Running, and final B termination/disappearance. Only temporary fixture tasks were stopped. Development bundle updated without restarting the user's active app. Temporary preview bundle and data were removed after closing.

### Threads header collapse control

ZedX's Threads search header now includes the existing sidebar close/toggle component at its right edge, with a keyboard-focusable Hide threads sidebar label. It reuses MultiWorkspace::close_sidebar and the sidebar-side context menu; upstream footer behavior remains intact. The pointer regression now checks the header control, hidden state and reopening/history access instead of the removed rail duplicate. Development build and whitespace checks passed. Regression case updated but not run; isolated native preview verified opening with Cmd+Alt+J and collapsing through the new header icon. Dev bundle updated; active user app not restarted. Temporary preview closed and removed.

### Collapsible Git commit information

The ZedX commit-details message section has a keyboard-focusable disclosure arrow and a compact subject header. It hides author/time/hash and Markdown when collapsed, releasing height to the changed-file tree; expansion restores the existing message entity and scroll handle. State belongs to the GitGraph instance and survives selecting another commit in that view. An 80px expanded minimum keeps the message viewport usable in a compact tool window. Upstream GitGraph only gains the state field/default; layout stays in `zedx_git_graph_details.rs`.

Validation: development build and whitespace check passed. The rendered compact-details regression now covers collapse/expand, metadata/message visibility and increased file-list height; written but not run. An isolated disposable Git fixture verified expanded author/message visibility, collapse to a subject row, larger file area and re-expansion. Only the fixture repository received a local validation commit. Checkout development bundle updated without restarting the active user app; temporary preview closed and removed.

### Commit selection work and Structure scrolling

`zedx_git_detail_loading.rs` owns the ZedX-specific commit diff completion path and changed-file tree cache. ZedX does not render line-level totals in its compact details, so it skips `compute_diff_stats` on the UI thread. Classic graph statistics retain upstream behavior. Completion checks selected SHA before publishing. The cache retains path/status entries and flattened tree across repaint; commit loads and directory expansion invalidate it. This removes repeated sorting/tree construction on hover or metadata notifications. Full commit content still loads through the existing asynchronous repository API; this is not a metadata-only Git implementation.

ZedX's OutlinePanel uses Auto uniform-list sizing and explicit zero minimum heights through its flex containers; Structure clips its remaining-height wrapper. The list viewport follows actual panel height rather than inferred content height. Classic list sizing remains Infer.

Validation: development build and diff whitespace checks passed. File-tree reuse/invalidation and real OutlinePanel bounded-viewport/wheel regression cases written, not run. Native isolated preview with 200 Markdown headings scrolled from Section 000 to Section 086, displaying a bounded scrollbar; a 20,000-line changed commit loaded its file/details without an observed UI stall. No numerical performance claim is made from CUA latency. A sample of the existing user process did not capture the suspected expensive diff-stat computation and is not treated as causal profiling evidence. Temporary preview closed and removed. Checkout dev bundle updated.

### Expose the native backdrop under light rails

The ZedX workspace root must stay transparent independently of `panel.background`. Making light panels white had caused the old alpha-based condition to repaint the workspace background under the rails, compounding the rail tint and obscuring the native backdrop. The root now tests ZedX mode only; white Project/editor content remains opaque. Light outer-frame/title/status colors share #F7F8FAB3 (about 70% opacity) instead of #F7F8FAFC. Existing macOS Blurred/NSVisualEffectView plumbing is retained; this change does not synthesize blur with a flat gray fill.

Validation: development build and whitespace checks passed. Light-frame alpha/white-content regression assertions were added but not run. An isolated native preview checked the resulting frame and opaque content areas; window-only captures are not used as proof of the appearance of desktop content behind the window. Temporary preview was closed and removed. Checkout dev bundle updated.

### Problems row layout

Owned `crates/zed/src/zedx_problem_row.rs` gives diagnostic rows a font-relative height, independent clipped message/location regions, and a single-line message summary with the full diagnostic in its tooltip. The panel applies the configured UI font. A regression covers long multiline diagnostics at normal and enlarged font sizes; written but not run. Development build passes.

### Run task controls

Owned `crates/zed/src/zedx_run_panel.rs` now provides scrollable task tabs, rerun/stop/clear/scroll-to-end controls, and a task-picker entry. It tracks task terminals in both the editor panes and Terminal dock, retains manual selection until a new task appears, and refreshes subscriptions when a task replaces its terminal model. Workspace pane snapshots avoid reading the leased workspace during panel initialization/activation. Rerun preserves native history/command parsing; after the native replacement reveals Terminal, Run restores its output view. Source regressions cover selection/removal and empty-view layout; not run. Development build passes. An isolated native preview verifies independent A/B output selection, stopping A with B still running, rerun staying in Run, and clear output leaving the process running.

### Recent run configurations

Owned `workspace/src/zedx_run_history.rs` persists ten deduplicated configuration identities and display labels per sorted project roots/remote connection in the local KV store. It caches reads and serializes writes. The task scheduling seam records usage; TitleBar restores the latest label/Run action and lists Recent Configurations above Choose/Edit. `tasks_ui` promotes currently resolved matching definitions into the recent section and handles recent clicks by resolving the current source/label identity, falling back to the picker when unavailable. Saved metadata contains no command/env snapshots; one-shot commands and ephemeral LSP tasks keep their existing session-only history. Shared guest projects are excluded. Source tests cover ordering, limits, serialization, DB reload and project isolation; not run. Build passes. Isolated native preview verifies menu MRU/deduplication, full process restart restoring the title/menu and chooser ranking, and executing a restored item.

### More menu controls tool button visibility

Owned `zedx_more_tools.rs` replaces the old open-panel menu with real solid checkboxes; the menu stays open for multiple changes. It only toggles rail buttons and leaves open panels/shortcuts intact. Owned `zedx_tool_visibility.rs` stores explicit visibility overrides in the profile KV store with serialized writes. Generic rails and the custom Notifications/AI/Outline/Database controls share the same preferences; Restore Default Layout clears overrides. Existing order preferences remain independent. Native preview confirms hiding Structure without changing Project, showing Branches, hiding right-side controls, and full restart retaining checkbox/button state. Build passes. Source tests cover visibility persistence/reset and menu/button behavior; existing Find navigation case now shows the pin before opening Find. Tests were not run.

### Recent Files navigation and status colors

Control+E opens Recent Files from workspace, editor and terminal contexts. Owned `crates/zed/src/zedx_recent_files_view.rs` supplies a responsive two-column popup: quick tool-window navigation on the left and recent files on the right. Left/Right switches keyboard focus between columns; Up/Down/Enter selects a tool and dismisses the popup before dispatch. File rows read current Git status, show IDEA blue for modified files and red for unversioned files, include status markers and an amber unsaved dot. Git-store observation refreshes the visible statuses.

Validation: development build and whitespace check passed; checkout development bundle updated without restarting the user's active app. Source regressions cover compact layout, column keyboard focus, action dispatch/dismissal and status mapping; written but not run. An isolated native fixture verified physical Control+E from an editor, keyboard navigation to Commit, modified/unversioned colors and unsaved indication. Temporary preview closed and removed.

### Single material layer for light sidebars

ZedX Light panel contents now share the transparent panel token with Dark, leaving the side dock as the sole material painter. The light dock tint is 62% opaque instead of 97%; the light outer frame/title/status tint is 42% opaque instead of 70%. Native blurred-window rendering remains enabled. Sticky tree overlays, compact overlay docks, bottom docks, editor and popup surfaces retain opaque backgrounds. Updated theme regressions cover transparent content, frame alpha and separate opaque overlays; tests were not run. Development build and whitespace checks passed. An isolated native preview checked the Project content and editor layout; window-only captures do not establish desktop blur appearance. Temporary preview closed and removed; checkout dev bundle updated without restarting the active user app.
