import * as vscode from 'vscode';
import { ChatViewProvider } from './providers/ChatViewProvider';
import { HistoryViewProvider } from './providers/HistoryViewProvider';
import { ContextViewProvider } from './providers/ContextViewProvider';
import { BrainWiresCLI } from './cli/BrainWiresCLI';
import { DiffManager } from './diff/DiffManager';

export function activate(context: vscode.ExtensionContext) {
  console.log('Brainwires extension is now active');

  // Initialize CLI interface
  const cli = new BrainWiresCLI();

  // Initialize managers
  const diffManager = new DiffManager();

  // Register webview providers
  const chatViewProvider = new ChatViewProvider(context.extensionUri, cli);
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider('brainwires.chatView', chatViewProvider)
  );

  const historyViewProvider = new HistoryViewProvider(context.extensionUri, cli);
  context.subscriptions.push(
    vscode.window.registerTreeDataProvider('brainwires.historyView', historyViewProvider)
  );

  const contextViewProvider = new ContextViewProvider(context.extensionUri);
  context.subscriptions.push(
    vscode.window.registerTreeDataProvider('brainwires.contextView', contextViewProvider)
  );

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('brainwires.chat', async () => {
      await vscode.commands.executeCommand('workbench.view.extension.brainwires-sidebar');
      chatViewProvider.focus();
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('brainwires.explainCode', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
      }

      const selection = editor.document.getText(editor.selection);
      if (!selection) {
        vscode.window.showErrorMessage('No code selected');
        return;
      }

      const fileName = editor.document.fileName;
      const language = editor.document.languageId;

      await chatViewProvider.sendMessage(
        `Explain this ${language} code from ${fileName}:\n\n\`\`\`${language}\n${selection}\n\`\`\``
      );

      await vscode.commands.executeCommand('workbench.view.extension.brainwires-sidebar');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('brainwires.refactorCode', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
      }

      const selection = editor.document.getText(editor.selection);
      if (!selection) {
        vscode.window.showErrorMessage('No code selected');
        return;
      }

      const instruction = await vscode.window.showInputBox({
        prompt: 'How would you like to refactor this code?',
        placeHolder: 'e.g., "Extract into a reusable function" or "Simplify logic"',
      });

      if (!instruction) {
        return;
      }

      const fileName = editor.document.fileName;
      const language = editor.document.languageId;

      await chatViewProvider.sendMessage(
        `Refactor this ${language} code from ${fileName}:\n\n${instruction}\n\n\`\`\`${language}\n${selection}\n\`\`\``
      );

      await vscode.commands.executeCommand('workbench.view.extension.brainwires-sidebar');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('brainwires.fixErrors', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
      }

      const diagnostics = vscode.languages.getDiagnostics(editor.document.uri);
      if (diagnostics.length === 0) {
        vscode.window.showInformationMessage('No errors found in this file');
        return;
      }

      const errors = diagnostics
        .filter((d) => d.severity === vscode.DiagnosticSeverity.Error)
        .map((d) => `Line ${d.range.start.line + 1}: ${d.message}`)
        .join('\n');

      const fileName = editor.document.fileName;

      await chatViewProvider.sendMessage(
        `Fix the following errors in ${fileName}:\n\n${errors}\n\nFile content:\n\`\`\`\n${editor.document.getText()}\n\`\`\``
      );

      await vscode.commands.executeCommand('workbench.view.extension.brainwires-sidebar');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('brainwires.generateTests', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
      }

      const selection = editor.document.getText(editor.selection);
      const codeToTest = selection || editor.document.getText();

      const fileName = editor.document.fileName;
      const language = editor.document.languageId;

      await chatViewProvider.sendMessage(
        `Generate comprehensive unit tests for this ${language} code from ${fileName}:\n\n\`\`\`${language}\n${codeToTest}\n\`\`\``
      );

      await vscode.commands.executeCommand('workbench.view.extension.brainwires-sidebar');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('brainwires.showDiff', async (filePath: string, originalContent: string, newContent: string) => {
      await diffManager.showDiff(filePath, originalContent, newContent);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('brainwires.acceptChanges', async () => {
      await diffManager.acceptChanges();
      vscode.window.showInformationMessage('Changes accepted');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('brainwires.rejectChanges', async () => {
      await diffManager.rejectChanges();
      vscode.window.showInformationMessage('Changes rejected');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('brainwires.openSettings', async () => {
      await vscode.commands.executeCommand('workbench.action.openSettings', 'brainwires');
    })
  );

  // Status bar item
  const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  statusBarItem.text = '$(comment-discussion) Brainwires';
  statusBarItem.tooltip = 'Open Brainwires Chat';
  statusBarItem.command = 'brainwires.chat';
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  // Show welcome message on first activation
  const hasShownWelcome = context.globalState.get('brainwires.hasShownWelcome', false);
  if (!hasShownWelcome) {
    vscode.window
      .showInformationMessage(
        'Welcome to Brainwires AI Assistant! Click the Brainwires icon in the sidebar to get started.',
        'Open Chat',
        'Settings'
      )
      .then((selection) => {
        if (selection === 'Open Chat') {
          vscode.commands.executeCommand('brainwires.chat');
        } else if (selection === 'Settings') {
          vscode.commands.executeCommand('brainwires.openSettings');
        }
      });

    context.globalState.update('brainwires.hasShownWelcome', true);
  }
}

export function deactivate() {
  console.log('Brainwires extension is now deactivated');
}
