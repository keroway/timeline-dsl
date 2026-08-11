/**
 * プレビュー用の純粋なヘルパー。
 *
 * **`vscode` を import しない。** ここを `preview.ts` に置くと、
 * `vscode` モジュールがテスト実行環境に存在しないためユニットテストから
 * 読めなくなる（実際に `Cannot find module 'vscode'` で落ちた）。
 * VS Code API に触る部分と、触らない部分を分けておく。
 */

import { execFile } from "node:child_process";

/** 1 回のレンダリングを打ち切るまでの時間。異常時に固まらないための上限。 */
const RENDER_TIMEOUT_MS = 10_000;

export type RenderResult =
  | { ok: true; svg: string }
  | { ok: false; message: string };

/**
 * `tdsl render --format svg` を単発実行して SVG を得る。
 *
 * 標準出力へ書かせるため `--output` は渡さない。エラー時は stderr を
 * そのまま返す（**無音で失敗させない**）。
 */
export function renderSvg(
  binaryPath: string,
  filePath: string,
): Promise<RenderResult> {
  return new Promise((resolve) => {
    execFile(
      binaryPath,
      ["render", filePath, "--format", "svg"],
      { timeout: RENDER_TIMEOUT_MS, maxBuffer: 32 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (error) {
          const detail = stderr.trim() || error.message;
          resolve({ ok: false, message: detail });
          return;
        }
        resolve({ ok: true, svg: stdout });
      },
    );
  });
}

/**
 * Webview に流し込む HTML を組み立てる。
 *
 * SVG は `tdsl` が生成した信頼できる出力だが、Webview の CSP は絞っておく
 * （`script-src` を許可しない）。インタラクティブ機能は WebUI 側の役割で、
 * ここは静的な表示に徹する。
 */
export function buildPreviewHtml(svg: string, cspSource: string): string {
  return `<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; img-src ${cspSource} data:; style-src 'unsafe-inline';">
<style>
  body { margin: 0; padding: 12px; background: var(--vscode-editor-background); }
  svg { max-width: 100%; height: auto; }
</style>
</head>
<body>${svg}</body>
</html>`;
}

/** エラー時に表示する HTML。**何も表示しないのではなく理由を出す。** */
export function buildErrorHtml(message: string): string {
  const escaped = message
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  return `<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<style>
  body { margin: 0; padding: 12px; font-family: var(--vscode-editor-font-family, monospace); }
  pre { white-space: pre-wrap; color: var(--vscode-errorForeground); }
</style>
</head>
<body><pre>${escaped}</pre></body>
</html>`;
}

