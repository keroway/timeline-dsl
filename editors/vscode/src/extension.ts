import * as fs from "node:fs";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import { resolveTdslBinary } from "./resolveBinary.js";

/** インストール手順を案内する URL */
const INSTALL_URL = "https://github.com/keroway/timeline-dsl#installation";

/** アクティブな LanguageClient。deactivate() で stop() するために保持する */
let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext): void {
  // --- 1. tdsl バイナリを解決 ---
  const configPath = vscode.workspace
    .getConfiguration("timelineDsl")
    .get<string>("serverPath", "");

  const resolved = resolveTdslBinary(
    configPath,
    process.env,
    (p) => fs.existsSync(p),
    process.platform,
  );

  if ("error" in resolved) {
    const msg =
      resolved.error === "configMissing"
        ? `Timeline DSL: \`timelineDsl.serverPath\` に指定されたパス "${configPath}" が見つかりません。`
        : "Timeline DSL: \`tdsl\` バイナリが PATH 上に見つかりません。";

    void vscode.window.showErrorMessage(
      `${msg} LSP 機能が無効化されます。\n` +
        `インストール手順: brew tap keroway/tap && brew install tdsl\n` +
        `詳細: ${INSTALL_URL}`,
      "インストール手順を開く",
    ).then((selection) => {
      if (selection === "インストール手順を開く") {
        void vscode.env.openExternal(vscode.Uri.parse(INSTALL_URL));
      }
    });
    // バイナリが解決できない場合は LanguageClient を起動しない
    return;
  }

  // --- 2. LanguageClient を起動 ---
  const serverOptions: ServerOptions = {
    run: {
      command: resolved.path,
      args: ["lsp"],
      transport: TransportKind.stdio,
    },
    debug: {
      command: resolved.path,
      args: ["lsp"],
      transport: TransportKind.stdio,
    },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "tdsl" }],
    synchronize: {
      // .tdsl ファイルの変更を LSP サーバに通知する
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.tdsl"),
    },
  };

  client = new LanguageClient(
    "timelineDsl",
    "Timeline DSL Language Server",
    serverOptions,
    clientOptions,
  );

  // context.subscriptions に登録することで拡張無効化時に自動的に stop() される
  context.subscriptions.push(client);
  void client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
