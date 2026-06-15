import * as vscode from "vscode";

export function activate(_context: vscode.ExtensionContext): void {
  // LanguageClient の起動は #470 で実装する。
  // 本ファイルは TypeScript ビルド基盤を成立させるためのスケルトン。
}

export function deactivate(): Thenable<void> | undefined {
  return undefined;
}
