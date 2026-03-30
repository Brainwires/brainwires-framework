import * as vscode from 'vscode';
import { spawn, ChildProcess} from 'child_process';

export interface CLIResponse {
  content: string;
  success: boolean;
  error?: string;
}

export class BrainWiresCLI {
  private cliPath: string;
  private process: ChildProcess | null = null;

  constructor() {
    this.cliPath = vscode.workspace.getConfiguration('brainwires').get('cliPath', 'brainwires');
  }

  async chat(message: string, context?: string[]): Promise<CLIResponse> {
    try {
      const config = vscode.workspace.getConfiguration('brainwires');
      const provider = config.get('provider', 'anthropic');
      const model = config.get('model', 'claude-3-5-sonnet-20241022');
      const permissionMode = config.get('permissionMode', 'auto');

      // Build command with context files
      let fullMessage = message;
      if (context && context.length > 0) {
        const contextContent = await this.readContextFiles(context);
        fullMessage = `${contextContent}\n\n${message}`;
      }

      // Execute CLI
      const result = await this.executeCLI([
        'task',
        fullMessage,
        '--provider', provider,
        '--model', model,
      ]);

      return {
        content: result,
        success: true,
      };
    } catch (error: any) {
      return {
        content: '',
        success: false,
        error: error.message || String(error),
      };
    }
  }

  async plan(taskDescription: string): Promise<CLIResponse> {
    try {
      const config = vscode.workspace.getConfiguration('brainwires');
      const provider = config.get('provider', 'anthropic');
      const model = config.get('model', 'claude-3-5-sonnet-20241022');

      const result = await this.executeCLI([
        'plan',
        taskDescription,
        '--provider', provider,
        '--model', model,
      ]);

      return {
        content: result,
        success: true,
      };
    } catch (error: any) {
      return {
        content: '',
        success: false,
        error: error.message || String(error),
      };
    }
  }

  async getCost(): Promise<string> {
    try {
      return await this.executeCLI(['cost']);
    } catch (error) {
      return 'Failed to get cost information';
    }
  }

  async getHistory(): Promise<string[]> {
    try {
      // This would need to be implemented in CLI to return JSON
      const result = await this.executeCLI(['history', '--json']);
      return JSON.parse(result);
    } catch (error) {
      return [];
    }
  }

  private async executeCLI(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
      const cwd = workspaceFolder?.uri.fsPath || process.cwd();

      this.process = spawn(this.cliPath, args, {
        cwd,
        shell: true,
      });

      let stdout = '';
      let stderr = '';

      this.process.stdout?.on('data', (data) => {
        stdout += data.toString();
      });

      this.process.stderr?.on('data', (data) => {
        stderr += data.toString();
      });

      this.process.on('close', (code) => {
        this.process = null;
        if (code === 0) {
          resolve(stdout);
        } else {
          reject(new Error(stderr || `CLI exited with code ${code}`));
        }
      });

      this.process.on('error', (error) => {
        this.process = null;
        reject(error);
      });
    });
  }

  private async readContextFiles(filePaths: string[]): Promise<string> {
    const contents: string[] = [];

    for (const filePath of filePaths) {
      try {
        const uri = vscode.Uri.file(filePath);
        const content = await vscode.workspace.fs.readFile(uri);
        const text = Buffer.from(content).toString('utf8');
        contents.push(`\n### File: ${filePath}\n\`\`\`\n${text}\n\`\`\`\n`);
      } catch (error) {
        // Skip files that can't be read
      }
    }

    return contents.length > 0
      ? `Context files:\n${contents.join('\n')}`
      : '';
  }

  async checkCLIAvailability(): Promise<boolean> {
    try {
      await this.executeCLI(['--version']);
      return true;
    } catch (error) {
      return false;
    }
  }

  stopCurrentProcess(): void {
    if (this.process) {
      this.process.kill();
      this.process = null;
    }
  }
}
