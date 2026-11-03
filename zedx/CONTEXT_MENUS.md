# IDEA-style context menus

Open **Settings → Keymap → Context Menus**, then select **Editor**, **Project**, **Git** or **Terminal**. Configuration stays inside the Settings page; context menus do not contain a customization footer.

- Check/uncheck an item to show/hide it.
- Select a row and use Move Up / Move Down to change its position. Separator rows can be moved or hidden too; leading, trailing and duplicate separators are removed automatically.
- Choose **Add Command…**, search registered commands, select one, choose its menu label, and optionally supply JSON arguments.
- Select a row and choose **Icon…** to preview and assign a symbol, choose **No Icon**, or restore **Default Icon**. This works for built-in and custom commands. Common commands include default symbols for Copy, Cut, Paste, New File/Folder, Rename, Delete, navigation, terminal, history and links.
- The trash button removes an added command. Built-in commands can be hidden.
- **Restore Defaults** resets the current menu's draft. **Save** persists it and leaves Settings open with a Saved status. **Revert Changes** returns to the last saved configuration without closing Settings.
- The four scope tabs retain separate editor drafts while switching between them.

Preferences remain under `zedx_context_menus` in user settings. Each scope is independent and saving preserves unrelated settings. Reopen a context menu to see changes. Existing callbacks, disabled states, target focus, checkmarks, submenu navigation and shortcut lookup are preserved. Extra commands unavailable in the target context remain disabled; duplicates retain the original handler and disabled state. Variable-height custom components keep their original height.

Common registered items are available in Settings even before opening a context menu. Actual menus provide their additional context-dependent entries when built. Available entries may vary with the selected file, project or repository. The old `zedx::CustomizeContextMenu` action redirects to this Settings page for compatibility with existing keymaps.

Advanced example:

```json
{
  "zedx_context_menus": {
    "editor": {
      "hidden": ["editor::Cut"],
      "order": ["editor::Copy", "editor::Paste", "custom:find_todo"],
      "items": [
        {
          "id": "find_todo",
          "label": "Find TODO in Project",
          "action": "zedx::FindInFiles",
          "arguments": { "query": "TODO" }
        }
      ]
    }
  }
}
```

Built-in action IDs use their registered command names; repeated occurrences have `#2`, `#3`, etc. Callback-only items use `label:…`, submenus use `submenu:…`, and custom entries use `custom:<id>`. Prefer the dialog for context-dependent headings and separators. Items absent from the current popup retain their stored preferences. Resetting one scope does not reset the others.

Menus use an 8px surface radius, 4px selection radius, one-pixel border, theme-aware shadow/surface colors, a two-rem row height (28px at a 14px UI font), a shared icon gutter and unboxed shortcut text. The `ZEDX_UPSTREAM_UI` escape hatch retains upstream appearance and behavior.

The menu icon increment has passed a development build. The Settings page, icon picker, hide preview and reverting drafts were checked manually. Regression tests were added but not run.

Icon overrides are stored per scope under `icons`, keyed by the same stable item IDs used for ordering. Names use snake_case (for example `copy`, `pencil`, `terminal`); an empty string hides the icon. Removing a key restores the original icon or the default command symbol. Configuration uses packaged icons; it does not load external files or URLs.

```json
{"zedx_context_menus":{"project":{"icons":{"project_panel::Copy":"copy","label:Open in Terminal":"terminal","project_panel::Paste":""}}}}
```
