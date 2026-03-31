# TUI Keyboard Shortcuts Reference

## Overview
The brainwires-cli TUI supports a comprehensive set of keyboard shortcuts for efficient text editing and navigation, similar to standard terminal editors and IDEs.

## Multi-line Input

| Shortcut | Action |
|----------|--------|
| **Alt+Enter** | Insert newline (most reliable) |
| **Ctrl+J** | Insert newline (Unix standard) |
| **Shift+Enter** | Insert newline (if terminal supports) |
| **Enter** | Submit message |

## Cursor Movement

### Character Navigation
| Shortcut | Action |
|----------|--------|
| **Left Arrow** | Move cursor left one character |
| **Right Arrow** | Move cursor right one character |

### Word Navigation
| Shortcut | Action |
|----------|--------|
| **Alt+Left** or **Ctrl+Left** | Move cursor to start of previous word |
| **Alt+Right** or **Ctrl+Right** | Move cursor to start of next word |

### Line Navigation
| Shortcut | Action |
|----------|--------|
| **Home** | Move to start of input (first line, first character) |
| **End** | Move to end of input (last line, last character) |
| **Ctrl+A** | Move to start of current line |
| **Ctrl+E** | Move to end of current line |

### Document Navigation
| Shortcut | Action |
|----------|--------|
| **Up Arrow** | When input focused: Navigate history up<br/>When conversation focused: Scroll conversation up |
| **Down Arrow** | When input focused: Navigate history down<br/>When conversation focused: Scroll conversation down |
| **PgUp** | Scroll conversation up (page) |
| **PgDn** | Scroll conversation down (page) |

## Text Deletion

### Character Deletion
| Shortcut | Action |
|----------|--------|
| **Backspace** | Delete character before cursor |
| **Delete** | Delete character after cursor |

### Word Deletion
| Shortcut | Action |
|----------|--------|
| **Alt+Backspace** | Delete word before cursor |
| **Ctrl+W** | Delete word before cursor (Unix standard) |
| **Alt+Delete** | Delete word after cursor |

### Line Deletion
| Shortcut | Action |
|----------|--------|
| **Ctrl+U** | Delete from cursor to start of current line |
| **Ctrl+K** | Delete from cursor to end of current line |

## Application Controls

### View Switching
| Shortcut | Action |
|----------|--------|
| **Tab** | Switch focus between conversation and input panels |
| **Tab** (in conversation) | Enter fullscreen conversation mode |
| **Ctrl+L** | Open session picker |
| **Ctrl+D** | Toggle console view (debug messages) |
| **Ctrl+R** | Open reverse search (history search) |
| **Ctrl+F** | Open file explorer (from fullscreen mode) |
| **Ctrl+G** | Open Git SCM (from fullscreen mode) |
| **Ctrl+T** | Open task viewer |
| **Ctrl+B** | Open sub-agent viewer |

### Autocomplete
| Shortcut | Action |
|----------|--------|
| **/** (slash) | Show available slash commands |
| **Up/Down** | Navigate autocomplete suggestions |
| **Tab** | Accept suggestion and add space |
| **Enter** | Accept suggestion and submit |

### Session Management
| Shortcut | Action |
|----------|--------|
| **Ctrl+Z** | Open background/suspend dialog |

When the background dialog appears:
- **Background**: Detaches TUI, keeps Agent running in background. Use `brainwires attach` to reconnect later.
- **Suspend**: Suspends the TUI process (like terminal `Ctrl+Z`), resumes with `fg`.
- **Cancel**: Closes the dialog.

### Application
| Shortcut | Action |
|----------|--------|
| **Ctrl+C** | Quit application (also shuts down Agent) |
| **Esc** | Exit current overlay/mode |

## Context-Specific Shortcuts

### Session Picker Mode
| Shortcut | Action |
|----------|--------|
| **Up/Down** | Navigate sessions |
| **Enter** | Load selected session |
| **Esc** | Cancel |

### Reverse Search Mode
| Shortcut | Action |
|----------|--------|
| **Type** | Filter history |
| **Up/Down** | Navigate results |
| **Enter** | Select result |
| **Esc** | Cancel |

### Console View Mode
| Shortcut | Action |
|----------|--------|
| **PgUp/PgDn** | Scroll console |
| **Esc** | Exit console view |
| **Ctrl+D** | Toggle console view |

### File Explorer Mode (Ctrl+F from fullscreen)
| Shortcut | Action |
|----------|--------|
| **Up/Down** | Navigate entries |
| **Enter** | Open directory / Edit file |
| **Space** | Toggle file selection |
| **Left/Backspace** | Go to parent directory |
| **/** | Start search/filter |
| **.** | Toggle hidden files |
| **i** | Insert selected files to AI context |
| **e** | Edit current file |
| **Esc** | Close file explorer |

### Nano Editor Mode
| Shortcut | Action |
|----------|--------|
| **Arrow Keys** | Move cursor |
| **Ctrl+S** | Save file |
| **Ctrl+X** | Exit editor |
| **Ctrl+K** | Cut line |
| **Ctrl+U** | Paste |
| **Home/End** | Line start/end |
| **PgUp/PgDn** | Page navigation |
| **Esc** | Exit (warns if unsaved) |

### Git SCM Mode (Ctrl+G from fullscreen)
| Shortcut | Action |
|----------|--------|
| **Tab** | Switch between panels |
| **Up/Down** | Navigate file list |
| **Space** | Toggle file selection |
| **s/Enter** | Stage selected file(s) |
| **u** | Unstage selected file(s) |
| **d** | Discard changes (with confirm) |
| **c** | Start commit |
| **P** (uppercase) | Push to remote |
| **p** (lowercase) | Pull from remote |
| **f** | Fetch from remote |
| **r** | Refresh status |
| **Esc** | Close Git SCM |

### Journal Tree Navigation (conversation focused, Journal view)
| Shortcut | Action |
|----------|--------|
| **j** / **Down** | Move cursor to next node |
| **k** / **Up** | Move cursor to previous node |
| **l** / **Right** | Expand selected node |
| **h** / **Left** | Collapse selected node (or move cursor to parent) |
| **Enter** / **Space** | Toggle collapse/expand on selected node |
| **g** | Jump to first node |
| **G** | Jump to last node |

### Sub-Agent Viewer (Ctrl+B)
| Shortcut | Action |
|----------|--------|
| **Tab** | Switch between agent list (left) and detail (right) panels |
| **j** / **Up** | Navigate up in focused panel |
| **k** / **Down** | Navigate down in focused panel |
| **Enter** (left panel) | Select agent and focus detail panel |
| **Type** (right panel, IPC available) | Compose message to send to agent |
| **Backspace** (right panel) | Delete last character of composed message |
| **Enter** (right panel, message non-empty) | Send message to agent via IPC |
| **Ctrl+B** / **Esc** | Close sub-agent viewer |

## Platform Compatibility

### Why Multiple Key Bindings?

Many shortcuts have alternative key combinations for cross-platform compatibility:

- **Alt+Enter vs Ctrl+J**: Terminal emulators often don't send Shift+Enter properly, so we support Alt+Enter (most reliable) and Ctrl+J (Unix standard)
- **Alt+Left/Right vs Ctrl+Left/Right**: Some platforms use Alt, others use Ctrl for word navigation
- **Alt+Backspace vs Ctrl+W**: macOS typically uses Alt (Option), while Unix/Linux uses Ctrl+W

### Terminal Compatibility

The TUI has been tested and works with:
- **macOS Terminal**
- **iTerm2**
- **GNOME Terminal**
- **Konsole**
- **Alacritty**
- **kitty**
- **Windows Terminal**
- **tmux** (with appropriate terminal settings)

## Tips and Tricks

1. **Efficient Editing**: Use Ctrl+A and Ctrl+E to quickly move within lines instead of holding arrow keys
2. **Quick Deletion**: Use Alt+Backspace to delete mistakes word-by-word instead of character-by-character
3. **Line Editing**: Use Ctrl+U to clear the current line and start over
4. **Focus Switching**: Press Tab to switch between conversation and input, then use arrow keys to scroll or navigate
5. **Multi-line Composition**: Use Alt+Enter to compose multi-line messages with proper formatting
6. **History Search**: Press Ctrl+R and start typing to fuzzy-search your command history

## Markdown Support

The TUI supports full markdown formatting in messages:

- **Headers**: `# H1`, `## H2`, etc.
- **Bold**: `**bold text**`
- **Italic**: `*italic text*`
- **Code**: `` `inline code` ``
- **Code Blocks**: Triple backticks
- **Lists**: `- item` or `1. item`

Both user and assistant messages render with colored markdown formatting for better readability.

## See Also

- [File Explorer & Git SCM](./TUI_FILE_EXPLORER_GIT_SCM.md) - Detailed documentation for file and git tools
- [TUI Multi-line and Markdown Fixes](./TUI_MULTILINE_MARKDOWN_FIXES.md) - Implementation details
- [Sessions](../SESSIONS.md) - Session lifecycle, socket architecture, and sub-agent sessions
- [Slash Commands](../README.md#slash-commands) - Available commands
