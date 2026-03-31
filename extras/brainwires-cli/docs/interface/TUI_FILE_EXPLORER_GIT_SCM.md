# TUI File Explorer & Git Source Control

## Overview

The brainwires-cli TUI includes two powerful full-screen tools for file management and version control:

1. **File Explorer** - Browse files, add them to AI context, and edit with the built-in nano-style editor
2. **Git SCM** - Full Git integration with staging, committing, pushing, pulling, and more

Both tools are accessible from the fullscreen conversation view (`Tab` twice from normal mode).

---

## File Explorer

### Accessing the File Explorer

1. Enter fullscreen conversation mode: Press `Tab` twice from normal input mode
2. Open File Explorer: Press `Ctrl+F`

### Interface Layout

```
+------------------------------------------------------------------+
| File Explorer - /home/user/project/src                           |
|------------------------------------------------------------------|
|  .. (Parent Directory)                                           |
|  [DIR]  components/                                              |
|> [x]    main.rs                                       12.5 KB    |
|  [ ]    lib.rs                                         4.2 KB    |
|  [ ]    utils.rs                                       2.1 KB    |
|------------------------------------------------------------------|
| ^v:Move  Enter:Open  Space:Select  i:Insert  e:Edit  Esc:Close   |
+------------------------------------------------------------------+
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| **Up/Down** | Navigate entries |
| **PgUp/PgDn** | Page navigation |
| **Enter** | Open directory / Edit file |
| **Space** | Toggle file selection |
| **Left/Backspace** | Go to parent directory |
| **Right** | Enter selected directory |
| **/** | Start search/filter |
| **.** | Toggle hidden files |
| **a** | Select all files |
| **n** | Clear all selections |
| **i** | Insert selected files to AI context |
| **e** | Edit current file in nano editor |
| **r** | Refresh file list |
| **Esc** | Close file explorer |

### Features

#### Multi-Select Files
Use `Space` to toggle selection on files. Selected files show `[x]` next to them. Use `a` to select all files in the current directory or `n` to clear selections.

#### Insert Files to Context
After selecting files with `Space`, press `i` to add them to the AI's working context. This allows the AI to reference file contents in conversations.

#### Search/Filter
Press `/` to enter search mode. Type to filter files by name. Press `Enter` to confirm or `Esc` to cancel.

#### File Size Display
File sizes are shown in human-readable format (KB, MB) next to each file entry.

---

## Nano-Style Editor

### Accessing the Editor

From the File Explorer:
- Press `Enter` on a file to open it
- Press `e` on a selected file

### Interface Layout

```
+------------------------------------------------------------------+
| Nano Editor - src/main.rs - Modified                             |
|------------------------------------------------------------------|
|   1 | //! Main entry point                                       |
|   2 |                                                             |
|   3 | use anyhow::Result;                                        |
|   4 | mod cli;|  <-- cursor                                       |
|   5 |                                                             |
|------------------------------------------------------------------|
| ^S:Save  ^X:Exit  ^K:Cut  ^U:Paste       Ln:4 Col:8              |
+------------------------------------------------------------------+
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| **Arrow Keys** | Move cursor |
| **Home/End** | Move to line start/end |
| **PgUp/PgDn** | Page navigation |
| **Ctrl+S** | Save file |
| **Ctrl+O** | Write out (save, nano-style) |
| **Ctrl+X** | Exit editor |
| **Ctrl+K** | Cut current line |
| **Ctrl+U** | Paste cut line |
| **Backspace** | Delete character before cursor |
| **Delete** | Delete character after cursor |
| **Enter** | Insert newline |
| **Tab** | Insert tab character |
| **Esc** | Exit (with unsaved changes warning) |

### Features

#### Line Numbers
Line numbers are displayed on the left side for easy navigation and reference.

#### Modified Indicator
The title bar shows "Modified" when there are unsaved changes.

#### Large File Warning
Files larger than 1MB are opened in read-only mode with a warning.

#### Binary File Detection
Files containing null bytes are detected as binary and opened read-only.

---

## Git Source Control (SCM)

### Accessing Git SCM

1. Enter fullscreen conversation mode: Press `Tab` twice from normal input mode
2. Open Git SCM: Press `Ctrl+G`

**Note:** Git SCM only works when in a Git repository. If not in a repo, you'll see an error message.

### Interface Layout

```
+------------------------------------------------------------------+
|  main -> origin/main (^2 v1)                                     |
|  3 file(s) changed                                               |
+------------------------------------------------------------------+
| Staged (1)       | Changes (2)       | Untracked (0)             |
|------------------|-------------------|---------------------------|
| [x] M  config.rs | [ ] M  main.rs    | No files                  |
|                  |>[ ] M  lib.rs     |                           |
|                  |                   |                           |
+------------------------------------------------------------------+
| Tab:Panel Space:Select Enter/s:Stage u:Unstage d:Discard         |
| c:Commit P:Push p:Pull f:Fetch r:Refresh Esc:Close               |
+------------------------------------------------------------------+
```

### Header Information

- **Branch name** - Current branch (e.g., `main`)
- **Upstream tracking** - Remote tracking branch (e.g., `origin/main`)
- **Ahead/Behind** - Number of commits ahead (`^`) and behind (`v`) remote
- **Total changes** - Count of all changed files

### Three-Panel Layout

| Panel | Description | Status Codes |
|-------|-------------|--------------|
| **Staged** | Files staged for commit | `M ` (modified), `A ` (added), `D ` (deleted), `R ` (renamed) |
| **Changes** | Modified but not staged | ` M` (modified), ` D` (deleted), `UU` (conflict) |
| **Untracked** | New files not in Git | `??` |

### Keyboard Shortcuts

#### Navigation
| Shortcut | Action |
|----------|--------|
| **Tab** | Switch between panels |
| **Up/Down** | Navigate file list |
| **PgUp/PgDn** | Page navigation |

#### Selection
| Shortcut | Action |
|----------|--------|
| **Space** | Toggle file selection |

#### Git Operations
| Shortcut | Action |
|----------|--------|
| **s** or **Enter** | Stage selected/current file(s) |
| **u** | Unstage selected/current file(s) |
| **d** | Discard changes (with confirmation) |
| **c** | Start commit (opens commit message input) |
| **P** (uppercase) | Push to remote |
| **p** (lowercase) | Pull from remote |
| **f** | Fetch from remote |
| **r** | Refresh status |
| **Esc** | Close Git SCM |

### Commit Mode

When you press `c` to commit (with staged files):

```
+------------------------------------------------------------------+
|                    Commit Message                                 |
|------------------------------------------------------------------|
| 3 file(s) to commit                                              |
|                                                                  |
| Add new feature implementation_                                   |
|                                                                  |
| Enter to commit, Esc to cancel                                   |
+------------------------------------------------------------------+
```

- Type your commit message
- Press `Enter` to commit
- Press `Esc` to cancel

### Confirmation Dialogs

Destructive operations (like discarding changes) show a confirmation:

```
Discard changes to 2 file(s)? (y/n)
```

- Press `y` to confirm
- Press `n` or `Esc` to cancel

### Status Messages

Success and error messages appear in the header area:
- Green checkmark for success (e.g., "Committed successfully")
- Red X for errors (e.g., "Push failed: Authentication required")

---

## Workflow Examples

### Adding Files to AI Context

1. Press `Tab` twice to enter fullscreen mode
2. Press `Ctrl+F` to open File Explorer
3. Navigate to files you want to add
4. Press `Space` to select multiple files
5. Press `i` to insert them to context
6. Press `Esc` to return to conversation

### Quick Edit and Commit

1. Press `Tab` twice to enter fullscreen mode
2. Press `Ctrl+F` to open File Explorer
3. Navigate to file and press `Enter` to edit
4. Make changes, press `Ctrl+S` to save
5. Press `Ctrl+X` to exit editor
6. Press `Esc` to close File Explorer
7. Press `Ctrl+G` to open Git SCM
8. Press `s` to stage the modified file
9. Press `c`, type commit message, press `Enter`
10. Press `P` to push (optional)

### Review Changes Before Commit

1. Press `Ctrl+G` from fullscreen mode
2. Use `Tab` to navigate to "Changes" panel
3. Review modified files
4. Press `Space` to select files to stage
5. Press `s` to stage selected files
6. Press `c` to commit with message
7. Press `Esc` to close

---

## Tips and Best Practices

1. **Use File Explorer for Context** - Instead of manually copying file contents, use the File Explorer's `i` command to efficiently add files to the AI context.

2. **Check Status Before Committing** - Use `r` in Git SCM to refresh and see the latest status before committing.

3. **Stage Incrementally** - Use `Space` to select specific files and stage them in logical groups for cleaner commit history.

4. **Review the Header** - The Git SCM header shows ahead/behind counts - pull before pushing if you're behind.

5. **Use Discard Carefully** - The `d` command requires confirmation, but still be careful as it permanently discards changes.

---

## See Also

- [TUI Keyboard Shortcuts](./TUI_KEYBOARD_SHORTCUTS.md) - General TUI navigation
- [CLI Chat Modes](./CLI_CHAT_MODES.md) - Different chat interaction modes
