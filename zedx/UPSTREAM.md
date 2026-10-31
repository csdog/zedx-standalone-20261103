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
