# ZedX Architecture

A technical orientation for an engineer joining this codebase. It assumes you know Rust and have
seen a GUI framework before, but nothing about Zed, GPUI, or this fork.

Everything below is derived from the code in this checkout, not from upstream documentation. Where a
statement is an inference rather than something I measured, it says so.

---

## 1. What this is

ZedX is a **fork of the Zed editor** that reworks the UI into an IntelliJ IDEA–style workbench:
a left activity rail, tool windows in left/right/bottom docks, a project-avatar title bar, and a
IDE-style "Search Everywhere" modal opened by double-tapping Shift.

| | |
|---|---|
| Upstream baseline | `zed-industries/zed` **v1.26.2**, commit `c427327` (recorded in `zedx/upstream.json`) |
| Package | `crates/zed`, package name stays `zed`; the **binary is renamed `zedx`** |
| Toolchain | pinned `1.98.1` (`rust-toolchain.toml`) |
| Size of the fork's own code | **7,369 lines** across 25 `zedx_*` modules (includes ~2,300 lines of tests) |
| Upstream files touched | 12 at the fork commit (~137 changed lines), plus a handful of later seams |
| Test count today | 65 (`zed` bin) + 2 (`title_bar`) + 5 (Python asset tests) |
| Repo state | **shallow clone, 2 commits, large uncommitted working tree** — see §9 |

The fork is not a rewrite. Roughly 99% of the editor, GPUI, language servers, terminal, git, and
collaboration stack is untouched upstream code. The fork's job is to **shape the shell and add a few
IDEA-flavoured surfaces**, then stay mergeable.

---

## 2. Repo orientation: what is upstream and what is ours

Two naming conventions carry all the signal:

- **`zedx_*` modules and `script/zedx` belong to the fork.** If a file starts with `zedx`, it is ours
  and you may restructure it freely.
- **Everything else is upstream Zed.** Treat it as read-only unless you are adding a deliberate seam.

```
crates/
  zed/src/zedx_*.rs             fork entry points, panels, search modal, status items
  workspace/src/zedx_*.rs       workbench shell, rails, layout, compact mode
  ui/src/zedx*.rs               icon set, island corners, tool-window chrome
  title_bar/src/zedx_*.rs       project avatar, top-bar actions
  picker/src/zedx_embedded.rs   sizing override for the embedded search modal
  file_icons/src/zedx_file_icons.rs
  release_channel/src/zedx_brand.rs
assets/
  icons/zedx/*.svg              39 glyphs x light/dark = 78 files
  themes/zedx/zedx.json         paired ZedX Light/Dark theme
prototype/index.html            single-file HTML design reference (source of truth for UI)
zedx/                           fork docs: README, UPSTREAM.md, PROTOTYPE_PARITY.md, scripts
docs/                           PLAN.md (phases P0-P4), DESIGN.md (visual spec)
```

Docs worth reading before code:

1. `zedx/README.md` — what ZedX is and the command surface.
2. `docs/PLAN.md` — phase plan and acceptance records.
3. **`zedx/PROTOTYPE_PARITY.md`** — the live requirement→evidence matrix. If you want to know what is
   actually done, read this, not the README.
4. **`zedx/UPSTREAM.md`** — running log of every upstream seam and why it exists. Update it whenever
   you touch an upstream file.

---

## 3. The one architectural idea: thin seams + owned modules

Almost every visible feature follows the same shape:

> Put the behaviour in a new `zedx_*` module. Add the smallest possible hook to the upstream file so
> the module can observe and render. Keep the hook to a few lines.

That constraint exists because this is a **fork of a fast-moving upstream**. Every line you add to an
upstream file is a line you will resolve by hand on the next merge.

The current seams, as of this checkout:

| Layer | Owned module | Upstream hook |
|---|---|---|
| Identity | `release_channel/zedx_brand.rs`, `paths::APP_NAME` | `release_channel/lib.rs` name/app-id/update policy (~5 lines) |
| Visual kit | `ui/src/zedx.rs`, `zedx_island.rs`, `file_icons/zedx_file_icons.rs` | one `pub mod` in `ui.rs` (~1 line), three icon lookups in `file_icons.rs` |
| Workbench shell | `workspace/zedx_workbench.rs` (437 lines) | `workspace.rs`: 4 `mod` lines, a `[Entity<PanelButtons>; 3]` field, one `match` arm, three offset subtractions |
| Rail / dock rendering | `workspace/zedx_tool_window_bar.rs` | `dock.rs`: `#[path]` child module + a `if zedx_workbench::enabled()` render branch |
| Compact mode | `workspace/zedx_compact.rs` | one `mod` line; observers attached from owned code |
| Layout reset | `workspace/zedx_layout.rs` | one `mod` line |
| Bottom bar | `workspace/zedx_bottom_bar.rs` | one `mod` line |
| Search | `zed/zedx_search.rs` + `view`/`policy`/`paths`/`gesture` | `main.rs` 2 lines, `zed.rs` 1 binding call |
| Panels | `zed/zedx_panels.rs`, `zedx_run_panel.rs`, `zedx_find_panel.rs` | `main.rs` 2 lines, `zed.rs` 1 join into an init future |
| Status bar | `zed/zedx_status_bar.rs` | `zed.rs`: create two items + register them (4 lines) |
| Picker sizing | `picker/zedx_embedded.rs` | `picker.rs`: one `mod` line |
| Git history | `git_ui/git_panel.rs` | `pub fn show_history` + a `test-support` accessor |

Two properties matter here:

- **The seams are additive.** They call into owned code; owned code rarely calls upstream internals.
  That keeps a merge conflict local and mechanical.
- **Some seams are unavoidable.** GPUI dispatch is structural: to receive key or modifier events a
  module must be *in the element tree*, which is why `workspace.rs` and `dock.rs` have those branches.

---

## 4. Layer map and dependency direction

```
             ┌──────────────────────────┐
             │ crates/zed (binary zedx) │  panels, search modal, status items
             └───────┬───────────┬──────┘
                     │           │
      ┌──────────────▼──┐   ┌────▼──────────────┐
      │ workspace       │   │ title_bar         │
      │ workbench/rails │   │ identity/actions  │
      └───────┬─────────┘   └────┬──────────────┘
              │                  │
        ┌─────▼──────────────────▼─────┐
        │ ui (+ zedx icon kit/themes)  │
        └──────────────┬───────────────┘
                       │
                  ┌────▼────┐
                  │  gpui   │  upstream: elements, dispatch, text, platform
                  └─────────┘
```

Direction is strictly downward. `ui::zedx` knows about GPUI and themes only. `workspace::zedx_*`
knows about docks and panels. `zed::zedx_*` knows about everything and is where features live.
Do not make a lower layer depend on a higher one to "reuse" a helper — the fork's layers stay clean
and that is what makes the seams cheap.

Shared vocabulary you need for GPUI (upstream concept, used everywhere here):

- `Entity<T>` is a handle to state on the single UI thread; `cx.spawn` runs foreground async work,
  `cx.background_spawn` off-thread work.
- Elements are described declaratively (`div()`, `h_flex()`, `v_flex()`) and are flexbox-laid-out.
- Actions are typed structs dispatched along a **focus path**. An element only sees an action or key
  event if it is an ancestor of the focused node. This single rule explains most of the odd-looking
  wiring in this repo (see §7.5 and the `ActivateHistoryTab` note in §8.4).
- Panels are `Panel` trait implementors, hosted by a `Dock`, which is hosted by a `Workspace`.

---

## 5. How a frame is drawn

The fork replaces the workspace's centre-of-window layout, not the window.

```
Window
 └─ MultiWorkspace (upstream)
     └─ Workspace::render (upstream, ~10k lines)
         ├─ TitleBar                      fork: project avatar + IDEA top actions
         ├─ if zedx_workbench::enabled()  ← single switch, workspace.rs:9801
         │    └─ render_zedx_workbench()   fork: workbench.rs:157
         │         ├─ left rail  (LEFT_TOOL_BAR_WIDTH = 44px)
         │         │    ├─ PanelButtons for the left dock   (upstream widget, fork renderer)
         │         │    ├─ "Git history" action button
         │         │    └─ "More tool windows" popover      (fork, workbench.rs:189)
         │         ├─ left dock island (10px radius, 6px inset)
         │         ├─ centre: editor island + bottom dock island
         │         └─ right rail (RIGHT_TOOL_BAR_WIDTH = 42px)
         └─ StatusBar                      fork: two owned items + upstream items
```

`zedx_workbench::enabled()` reads `ZEDX_UPSTREAM_UI` once via `OnceLock` (`zedx_workbench.rs:8`).
Setting that env var renders **the unmodified upstream layout**, which is the fork's comparison mode
(`script/zedx upstream-ui`). It is a debugging aid, not a user setting — see §8.6 for what that costs.

The "islands" look (translucent frame, opaque editor, rounded panels) is implemented three ways
because GPUI's model forces it:

1. `assets/themes/zedx/zedx.json` supplies translucent/opaque colours and enables the native blur.
2. Layout code paints 6px insets and 10px radii on each island.
3. **`ui::zedx::island_corners`** paints the corners explicitly, because GPUI masks children with
   rectangles — a rounded background alone cannot round its children's contents.

---

## 6. Subsystem tours

### 6.1 Workbench shell — `workspace/src/zedx_workbench.rs`

`render_zedx_workbench` composes four islands from the workspace's three docks. Notable details:

- Sizes are constants at the top of the file (`LEFT_TOOL_BAR_WIDTH = 44`, `RIGHT_TOOL_BAR_WIDTH = 42`,
  bottom default 202px, compact breakpoint 560px, compact overlay 235px). The prototype's numbers are
  the source of truth (`docs/DESIGN.md`).
- **Compact mode**: below 560px the left/right docks become absolute overlays instead of consuming
  editor width, and the stored desktop widths are left untouched so restoring the window restores the
  desktop layout. `zedx_compact.rs` makes left and right overlays mutually exclusive and closes the
  left overlay when a file is activated.
- Resize is not the upstream drag handle. `zedx_resize_gap` adds a 6px handle with mouse drag,
  arrow-key resize (8px, Shift 24px), Home to reset, and double-click to reset; it reuses the upstream
  `DraggedDock` payload so sizes persist through the existing serialization path.
- Vertical drag math subtracts the frame inset (`workspace.rs:9770-9788`), otherwise the editor and
  the dock disagree by 6px.

### 6.2 Tool windows and the rail — `zedx_tool_window_bar.rs`, `zedx_workbench.rs`

The rail is not a list of hardcoded buttons. `PanelButtons::render_zedx` enumerates the real panels in
a dock, sorts them into an IDEA-ish order (Project, Find, Git, Structure / Run, Terminal, Problems,
Debug), filters by `Panel::enabled`, and derives each button's icon, label, badge and toggle action
from the panel itself. Consequences worth knowing:

- A newly registered panel appears automatically, in the right dock, with the right badge.
- The fork's `More tool windows` popover (`zedx_tool_window_entries` + `zedx_more_tools`) lists every
  enabled panel from **all three docks** under Left/Bottom/Right headings, checking the active one and
  dispatching that panel's own `toggle_action`. This is how you reach a bottom tool from the left rail,
  which the prototype requires.
- Right-clicking a rail button opens the upstream dock-position menu, so panels can be relocated.

### 6.3 Search Everywhere — `zed/src/zedx_search.rs` (+ `_view`, `_policy`, `_paths`, `_gesture`)

The largest owned feature (~1,071 lines backend + 430 lines chrome + 1,022 lines tests). Flow:

```
modifiers_changed (double Shift, pure state machine)
   └─ open() / open_category()
        └─ toggle_modal(SearchWindow { picker: Picker<SearchDelegate> })
             ├─ All / Classes / Files / Symbols / Actions / Text
             ├─ providers: fuzzy worktree paths, LSP workspace symbols,
             │             window actions, streamed project-text search
             └─ view: grouped rows, filter popover, optional source preview
```

Deliberate design points:

- **The gesture is a pure state machine** (`zedx_gesture.rs`, 5 unit tests). Rules: a tap is a Shift
  press→release ≤350ms, two releases ≤400ms apart, any other modifier resets, window deactivation
  resets, and any non-Shift keydown resets. That last rule is wired from the element tree
  (`capture_key_down(... .reset())`), which is subtle: it is the only thing that stops
  "Shift+A" from being read as a tap.
- **Policy is separated from rendering** (`zedx_search_policy.rs`): file-type classification,
  UTF-8-safe highlight indices, and Chinese action aliases. `zedx_search_paths.rs` applies filters
  *before* fuzzy top-N ranking, which is the difference between "Rust filter works" and "Rust filter
  works only if your file is in the top 100 by score".
- **Stale-result safety**: a generation counter plus a cancellation flag; a slow provider cannot
  publish into a newer query.
- **The modal is bare** (`render_bare`), with a transparent backdrop, so the prototype's "no dim/blur
  behind the palette" rule holds.
- **One upstream hack**: `picker/zedx_embedded.rs::fit_zedx_search` overrides picker sizing for
  `Presentation::Embedded` only. It subtracts chrome from the height budget so a short window caps the
  *list*, not the whole popover.

### 6.4 Panels — `zed/src/zedx_panels.rs` (625 lines), `zedx_run_panel.rs`, `zedx_find_panel.rs`

Two very different strategies live side by side:

- **Composition (preferred).** `Structure` embeds the upstream `OutlinePanel` and forwards its panel
  events. `Run` renders the upstream `TerminalView` owned by the terminal panel — it does not start a
  second process. `Branches` embeds the upstream `git_ui::branch_picker::BranchList` via its public
  `create_embedded`. `Git history` focuses the upstream `GitPanel` and switches to its History tab.
  Reuse here is not laziness; it is how the fork keeps language-server, terminal and git behaviour in
  sync with upstream for free.
- **Reimplementation (costly, deliberate).** `Find` (484 lines) is a new project-search panel rather
  than a re-skin of upstream `project_search`. It exists because the prototype's left-dock find
  experience differs structurally from upstream's, and wrapping upstream's view was judged harder than
  a small owned implementation. Be aware of the consequence: upstream improvements to project search
  do **not** reach this panel, and it does not have regex/case/whole-word/replace yet.

`fixed_panel!` (macro in `zedx_panels.rs`) generates the boring half of `Panel`: persistent name,
settings key, fixed dock position, default size, icon, action, activation priority. Adding a panel is
usually ~30 lines plus one `add_panel` call in `initialize`.

### 6.5 Status bar — `zed/src/zedx_status_bar.rs`

Two owned items: `ProjectState` (branch + save state) and `Indentation` (per-file tabs/spaces). The
non-obvious part is event sourcing: it subscribes to `GitStoreEvent` rather than relying on entity
notifications, because the entity does not change when a repository is discovered later. A regression
(`status_follows_repository_added_after_window_open_and_branch_changes`) exists because that bug
shipped once.

### 6.6 Branding and distribution

- `paths::APP_NAME = "ZedX"` isolates config/cache/data directories from a real Zed install.
- `release_channel::zedx_brand` supplies display name and application id; `poll_for_updates()` returns
  `false`, so the official updater never runs.
- **Distribution is incomplete.** The binary is `zedx`, but `script/bundle-mac`, `bundle-linux`,
  `bundle-windows.ps1`, `script/debug-cli` and `script/generate-action-metadata` still pass
  `--bin zed` and will fail. `crates/cli` is untouched: its command name is `zed` and its URL scheme is
  `zed://`, so it collides with a real Zed install. `zedx/dev_bundle.py` produces a local
  `target/debug/ZedX.app` with icon and version taken from the crate, which is enough to double-click,
  not enough to ship.

---

## 7. Design decisions and what they cost

| Decision | Why | Cost you will feel |
|---|---|---|
| Fork with thin seams instead of vendoring whole crates | Upstream moves weekly; seams keep merges mechanical | Some features cannot be reached without a seam at all (event dispatch, picker sizing) |
| IDE-style shell rendered from owned code | Prototype drives exact pixel behaviour | The fork now owns layout bugs; upstream layout fixes do not apply |
| Reimplementation for Find, composition elsewhere | Prototype's left-dock find differs structurally | Two search surfaces to maintain; feature drift risk |
| Dual mode via `ZEDX_UPSTREAM_UI` | Cheap upstream comparison during development | Doubles the paths that can rot; `OnceLock` makes it impossible to switch within a process, so tests can only cover one mode |
| Hardcoded design constants in each module | Fast iteration against the prototype | `560.0` appears in four files; changing the compact breakpoint is a multi-file edit |
| Chinese action aliases via action-name prefix matching | The user searches in Chinese (`主题`, `恢复布局`) | Brittle: upstream renames silently drop the alias |
| Test-only accessors added to upstream files (`showing_history_tab`) | Assert real state without exposing production API | Small permanent upstream surface |
| Embedded picker sizing hook | The prototype's popover geometry is not expressible upstream | A fork-specific branch inside `crates/picker` |

---

## 8. Testing strategy

The fork treats "it renders" as insufficient. Three tiers:

1. **Pure unit tests** for logic that has no UI: the gesture state machine (5), search policy
   (6 tests: filters, UTF-8 highlight boundaries, alias stability).
2. **GPUI integration tests** for behaviour. These drive real events through the real
   `MultiWorkspace` tree, resolve elements by `debug_selector`, and assert both state and geometry:
   ```rust
   cx.dispatch_action(ToggleBranches);
   cx.refresh().unwrap();
   let hide = cx.debug_bounds("zedx-branches-hide").expect("hide control");
   cx.simulate_click(hide.center(), gpui::Modifiers::none());
   ```
   65 of these run today (`script/zedx integration-test`), plus 2 in `title_bar`.
3. **Asset tests** in Python (`zedx/tests/test_visual_assets.py`): every glyph has light and dark
   SVGs, the manifest matches the Rust enum, the theme keeps the editor opaque, the panel defaults
   match the prototype.

Two conventions make this work and are worth copying:

- **`debug_selector` on anything a test needs to hit.** Buttons from `ui::ButtonLike` do not expose
  selectors, so the fork wraps them (`div().debug_selector(|| "zedx-more-tools".into())`) or puts the
  selector on the inner glyph — a click at that point still reaches the button underneath, because
  hit-testing is positional.
- **Mutation-check new tests.** Change the implementation to the wrong behaviour and confirm the test
  fails, then revert. Two real examples in `UPSTREAM.md`: dropping the bottom dock from the tool-window
  listing, and no-op'ing the Branches hide handler. The second attempt initially patched `Structure`'s
  identical one-line handler and produced a false pass — always verify *which* call site your patch
  touched.

### Commands

```sh
script/zedx check            # cargo check -p zed --bin zedx
script/zedx build            # debug binary at target/debug/zedx
script/zedx run .            # run against this repo
script/zedx bundle           # rebuild target/debug/ZedX.app (icon + version)
script/zedx app .            # bundle + open
script/zedx upstream-ui .    # same binary, unmodified upstream layout
script/zedx integration-test # GPUI suites (zed + title_bar)
script/zedx gesture-test     # 5 pure gesture tests (fast, standalone rustc)
script/zedx search-test      # 6 policy tests (fast, standalone rustc)
script/zedx assets-test      # Python asset suite
```

Known environment limits measured here:

- `script/clippy -p zed -p workspace` cannot complete on this machine: upstream `gpui` trips two
  `-D warnings` lints under the pinned toolchain (`profiler/journal.rs`, `geometry.rs`), and clippy
  invalidates the `webrtc-sys` build-script fingerprint, which then tries to download a prebuilt and
  fails on the local TLS chain. Workaround used in this repo: `cargo clippy -p <crate> --lib -- -D
  warnings -A deprecated -A clippy::approx_constant`, which passes for `ui`, `workspace`, `git_ui`.
- `cargo test` outside `script/zedx` re-triggers that download; always go through the script.

---

## 9. Risks, ranked

1. **The fork is not mergeable yet.** The repo is a **shallow clone with 2 commits**. There is no
   recorded merge rehearsal, no release branch, and 14 binary icon files are replaced in-place under
   `crates/zed/resources/` — those cannot be three-way merged. Until a real upstream merge is done
   once, every "we can keep up with upstream" assumption is untested.
2. **Distribution does not work.** Renamed binary vs. upstream bundle scripts, unforked `cli` crate and
   `zed://` scheme. This is the gap between "runs from target/debug" and "installed application".
3. **Duplicated surfaces will drift.** Find panel vs. upstream project search; ZedX palette vs. upstream
   command palette. Decide explicitly which upstream improvements you will not take.
4. **Dual-mode rot.** `ZEDX_UPSTREAM_UI` creates a second layout path that no test covers as a whole,
   and its `OnceLock` prevents in-process switching.
5. **No fork CI.** `.github/workflows` is entirely upstream's and would fail on the renamed binary.
   The only gate is the person running `script/zedx integration-test`.
6. **Uncommitted work dominates the tree.** Roughly 38 modified files, including binary icons and
   1,000+ lines of tests, currently sit uncommitted. Anything uncommitted is one `git checkout` away
   from gone.
7. **Privacy defaults are upstream's.** Telemetry and crash reporting are not disabled in the fork;
   `should_install_crash_handler` only skips the Dev channel. If you ever package a stable-channel
   build, it will report to Zed's infrastructure.
8. **Visual polish is asserted weakly.** Rounded corners, blur and contrast were verified by eye.
   Only geometry and presence are test-asserted.

---

## 10. Getting productive

### Reading order (about half a day)

1. `zedx/README.md`, `docs/DESIGN.md` — what the UI should look like.
2. `zedx/PROTOTYPE_PARITY.md` — what is actually done, row by row.
3. `prototype/index.html` — the reference implementation. It is plain HTML/CSS/JS; the `#more-tools-btn`
   block and the `shiftTap*` block are the two least obvious behaviours.
4. `workspace/src/zedx_workbench.rs` — the shell. Start at `render_zedx_workbench`.
5. `workspace/src/zedx_tool_window_bar.rs` — how rails derive from panels.
6. `zed/src/zedx_search.rs` — the biggest feature; read `init` first, then the delegate.
7. `zed/src/zedx_panels.rs` — the panel zoo and the `fixed_panel!` macro.

### Recipes

**Add a tool window.** Define a struct with `focus`/`zoomed` (plus your own state), implement
`activation_focus`, `on_active`, `badge`, invoke `fixed_panel!(Type, "Name", "SettingsKey",
DockPosition::Left, 240.0, IconName::GitBranch, ToggleAction, priority)`, implement `Render`, create it
in `zedx_panels::initialize`, and register `ToggleAction` in `zedx_actions.rs` + `zedx_panels::init`.
It then appears in the rail and in `More tool windows` automatically.

**Add an action and a key.** Declare it in `workspace/src/zedx_actions.rs` (`actions!` for a unit
action, `#[derive(Action)]` when it carries data), register a handler with
`workspace.register_action(...)` inside an `observe_new` block, and bind keys in
`zedx_search::bind_keys` (or the relevant module) with an explicit key-context such as
`"ZedXSearch > Editor"` so you do not steal a global binding.

**Add a GPUI regression.** Use the `setup()` helpers in `zedx_panels_tests.rs` (which build a real
`MultiWorkspace` over `FakeFs`); `cx.refresh()` before reading bounds; give new controls a
`debug_selector`; assert state *and* bounds; then mutation-check.

**Touch an upstream file.** Add the seam, keep it to a handful of lines, and append a bullet to
`zedx/UPSTREAM.md` describing the seam and why it cannot live in owned code.

### Conventions from the repo root (`AGENTS.md` / `.rules`)

- No `unwrap()`/`expect()` in production paths; propagate with `?` or `.log_err()`.
- No `mod.rs` module paths; use `#[path]` + descriptive filenames (this is why modules here are flat).
- Comments explain *why*, never narrate *what*.
- Prefer extending an existing file over creating a new one; avoid speculative options and helpers.
- In GPUI tests prefer GPUI executor timers over `smol::Timer`.
- Do not commit or push without explicit instruction — and note that the README already carries the
  review-marker lines that `AGENTS.md` requires, which a human must remove manually.

---

## 11. Where the work stands

Phase plan lives in `docs/PLAN.md`; the honest per-requirement status is
`zedx/PROTOTYPE_PARITY.md`. Summary as of this checkout:

- **Done and tested:** IDE shell with islands and rails; project avatar + top actions; six-category
  Search Everywhere with real file/LSP/action/text providers, filters, preview, split; Find panel;
  Structure / Problems / Run / Branches / Git history; bottom bar with icons, counts and clear/hide;
  status bar items; compact overlays; resize + persistence; layout reset; 39 icon pairs and a paired
  theme; branding isolation.
- **Partial:** the prototype's Git window (no branch tools, remote grouping, log filter chips or commit
  detail pane); tool-window chrome (Branches and Problems have the shared head, Structure/Find still
  have bespoke headers); distribution.
- **Not started:** rail drag-reordering with persisted order; upstream merge rehearsal; performance
  comparison; fork CI; signed packaging.

If you pick up from here, the highest-value next steps are, in order: **commit the working tree**,
**do one real upstream merge to learn the actual cost**, **then finish distribution** (bundle scripts
+ `cli` + URL scheme), **then** continue prototype parity.
