import * as path from "node:path";

/**
 * バイナリ解決の結果型。
 * - 成功時: `{ path: string }` — 使用するバイナリの絶対パス
 * - 失敗時:
 *   - `configMissing`: `timelineDsl.serverPath` が指定されているが存在しない
 *   - `notOnPath`: PATH 探索でも見つからなかった
 */
export type ResolveResult =
  | { path: string }
  | { error: "configMissing" | "notOnPath" };

/**
 * `tdsl` バイナリを解決する純関数。
 *
 * 解決の優先順位:
 * 1. `configPath` が非空ならその絶対パスを使用。ファイルが存在しなければ `configMissing`。
 * 2. 空の場合は `PATH` を走査して `tdsl`（Win32 では `tdsl.exe` / `tdsl.cmd`）を探索。
 *    見つからなければ `notOnPath`。
 *
 * @param configPath `timelineDsl.serverPath` 設定値（空文字列の場合は PATH 解決へ）
 * @param env `process.env` に相当する環境変数 Map（テスト時にモック可能）
 * @param exists ファイル存在チェック関数（テスト時にモック可能）
 * @param platform `process.platform` 相当の文字列（テスト時にモック可能）
 */
export function resolveTdslBinary(
  configPath: string,
  env: NodeJS.ProcessEnv,
  exists: (p: string) => boolean,
  platform: string,
): ResolveResult {
  // プラットフォームに合わせたパスユーティリティとデリミタを選択
  const p = platform === "win32" ? path.win32 : path.posix;
  const delimiter = platform === "win32" ? ";" : ":";

  // 1. 設定ファイルパスが指定されている場合
  if (configPath.trim() !== "") {
    if (exists(configPath)) {
      return { path: configPath };
    }
    return { error: "configMissing" };
  }

  // 2. PATH 走査
  const pathEnv = env["PATH"] ?? "";
  const pathDirs = pathEnv.split(delimiter).filter((d) => d !== "");

  const candidates =
    platform === "win32" ? ["tdsl.exe", "tdsl.cmd", "tdsl"] : ["tdsl"];

  for (const dir of pathDirs) {
    for (const name of candidates) {
      const full = p.join(dir, name);
      if (exists(full)) {
        return { path: full };
      }
    }
  }

  return { error: "notOnPath" };
}
