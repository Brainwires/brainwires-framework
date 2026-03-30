# Brainwires VS Code Extension

AI-powered coding assistant integrated with Brainwires CLI.

## Features

### Commands
- **Brainwires: Open Chat** (`Ctrl+Shift+B` / `Cmd+Shift+B`) - Open chat interface
- **Brainwires: Explain Selection** (`Ctrl+Shift+E` / `Cmd+Shift+E`) - Explain selected code
- **Brainwires: Refactor Selection** - Refactor selected code with custom instructions
- **Brainwires: Fix Errors in File** - Automatically fix errors shown in Problems panel
- **Brainwires: Generate Tests** - Generate unit tests for selection or entire file
- **Brainwires: Show Diff** - Preview changes before applying
- **Brainwires: Accept/Reject Changes** - Apply or discard suggested changes

### Sidebar Views
- **Chat View** - Interactive chat with the AI assistant
- **History View** - Previous conversations and sessions
- **Context Files** - Files included in AI context

### Context Menu Integration
Right-click on selected code to:
- Explain code
- Refactor code
- Generate tests

## Setup

### Prerequisites
1. Install Brainwires CLI: `npm install -g brainwires-cli`
2. Configure API keys for your chosen provider

### Installation
1. Install extension from VS Code Marketplace
2. Open VS Code settings (Ctrl+,)
3. Search for "Brainwires"
4. Configure:
   - Provider (anthropic, openai, google, ollama)
   - Model
   - Permission mode
   - CLI path (if not in PATH)

## Configuration

```json
{
  "brainwires.provider": "anthropic",
  "brainwires.model": "claude-3-5-sonnet-20241022",
  "brainwires.permissionMode": "auto",
  "brainwires.cliPath": "brainwires",
  "brainwires.autoSuggest": true,
  "brainwires.showDiffBeforeApply": true,
  "brainwires.contextFiles": []
}
```

## Usage

### Chat Interface
1. Click Brainwires icon in Activity Bar or press `Ctrl+Shift+B`
2. Type your question or request
3. Review AI response and suggested changes
4. Accept or reject changes

### Code Explanation
1. Select code in editor
2. Press `Ctrl+Shift+E` or right-click → "Brainwires: Explain Selection"
3. View explanation in chat sidebar

### Refactoring
1. Select code to refactor
2. Right-click → "Brainwires: Refactor Selection"
3. Enter refactoring instructions
4. Review diff and accept/reject changes

### Fix Errors
1. Open file with errors
2. Run "Brainwires: Fix Errors in File"
3. AI analyzes errors and suggests fixes
4. Review and apply changes

## Development

### Building
```bash
cd vscode-extension
npm install
npm run compile
```

### Testing
```bash
npm test
```

### Packaging
```bash
npm run package
```

This creates a `.vsix` file you can install locally or publish to the marketplace.

## Architecture

### Extension Entry Point
`src/extension.ts` - Main extension activation and command registration

### CLI Interface
`src/cli/BrainWiresCLI.ts` - Interface to Brainwires CLI
- Executes CLI commands
- Handles communication
- Manages context files

### Providers (To Implement)
- `src/providers/ChatViewProvider.ts` - Chat webview
- `src/providers/HistoryViewProvider.ts` - History tree view
- `src/providers/ContextViewProvider.ts` - Context files tree view

### Diff Management (To Implement)
- `src/diff/DiffManager.ts` - Diff preview and application

## Contributing

1. Fork the repository
2. Create a feature branch
3. Implement changes
4. Add tests
5. Submit pull request

## License

MIT
