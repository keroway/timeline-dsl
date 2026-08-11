import * as fs from "node:fs";
import * as vscode from "vscode";
import { PreviewController } from "./preview";
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

  registerCommands(context, resolved.path);
}

/**
 * コマンドを登録する（#754）。
 *
 * **バイナリが解決できているときだけ呼ぶ。** 解決できていない場合は
 * `activate` が早期 return しており、コマンドを登録しても「押しても
 * 何も起きない」状態になるため。
 */
function registerCommands(
  context: vscode.ExtensionContext,
  binaryPath: string,
): void {
  const preview = new PreviewController(binaryPath);
  context.subscriptions.push({ dispose: () => preview.dispose() });

  context.subscriptions.push(
    vscode.commands.registerCommand("timelineDsl.openPreview", async () => {
      const editor = vscode.window.activeTextEditor;
      // 無音で何も起きないのは禁止（#754）。理由を出す。
      if (!editor || editor.document.languageId !== "tdsl") {
        void vscode.window.showErrorMessage(
          "Timeline DSL: プレビューは .tdsl ファイルを開いた状態で実行してください。",
        );
        return;
      }
      if (editor.document.isUntitled) {
        void vscode.window.showErrorMessage(
          "Timeline DSL: プレビューには保存済みのファイルが必要です（`tdsl render` がパスを受け取るため）。",
        );
        return;
      }
      await preview.open(editor.document);
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("timelineDsl.restartServer", async () => {
      if (!client) {
        void vscode.window.showErrorMessage(
          "Timeline DSL: Language Server が起動していません。",
        );
        return;
      }
      await client.restart();
      void vscode.window.showInformationMessage(
        "Timeline DSL: Language Server を再起動しました。",
      );
    }),
  );

  // 保存時に再描画する。入力のたびではなく保存契機にするのは、
  // 子プロセスの起動回数を抑えるため（debounce も PreviewController 側にある）。
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => {
      preview.scheduleRefresh(doc);
    }),
  );
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
