import { describe, it } from "node:test";
import assert from "node:assert/strict";
import * as path from "node:path";
import { resolveTdslBinary } from "./resolveBinary.js";

// テスト用ヘルパー: 指定したパス集合のみ存在するとみなす exists 関数を返す
function makeExists(existingPaths: ReadonlySet<string>): (p: string) => boolean {
  return (p: string) => existingPaths.has(p);
}

describe("resolveTdslBinary", () => {
  // (a) serverPath 指定・ファイル存在 → そのパスを返す
  it("serverPath が指定されていてファイルが存在する場合、そのパスを返す", () => {
    const fixedPath = "/usr/local/bin/tdsl";
    const result = resolveTdslBinary(
      fixedPath,
      {},
      makeExists(new Set([fixedPath])),
      "linux",
    );
    assert.deepEqual(result, { path: fixedPath });
  });

  // (b) serverPath 指定・ファイル不在 → configMissing エラー
  it("serverPath が指定されていてファイルが存在しない場合、configMissing を返す", () => {
    const result = resolveTdslBinary(
      "/nonexistent/tdsl",
      {},
      makeExists(new Set()),
      "linux",
    );
    assert.deepEqual(result, { error: "configMissing" });
  });

  // (c) serverPath 空・PATH にあり → 解決できる
  it("serverPath が空で PATH にバイナリがある場合、そのパスを返す", () => {
    const expected = path.join("/usr/local/bin", "tdsl");
    const result = resolveTdslBinary(
      "",
      { PATH: "/usr/bin:/usr/local/bin" },
      makeExists(new Set([expected])),
      "linux",
    );
    assert.deepEqual(result, { path: expected });
  });

  // (d) serverPath 空・PATH になし → notOnPath エラー
  it("serverPath が空で PATH にバイナリがない場合、notOnPath を返す", () => {
    const result = resolveTdslBinary(
      "",
      { PATH: "/usr/bin:/usr/local/bin" },
      makeExists(new Set()),
      "linux",
    );
    assert.deepEqual(result, { error: "notOnPath" });
  });

  // (e) Win32 で tdsl.exe を解決できる
  it("win32 プラットフォームで tdsl.exe を優先解決する", () => {
    // win32 パス区切り（;）で PATH を組み立て、win32.join でフルパスを構築
    const expected = path.win32.join("C:\\tools\\tdsl", "tdsl.exe");
    const pathEnv = ["C:\\Windows\\System32", "C:\\tools\\tdsl"].join(";");
    const result = resolveTdslBinary(
      "",
      { PATH: pathEnv },
      // 存在チェックは win32 の join 結果と突き合わせる
      makeExists(new Set([expected])),
      "win32",
    );
    assert.deepEqual(result, { path: expected });
  });

  // (f) serverPath が空白文字のみ → PATH 解決に fallback する
  it("serverPath が空白文字のみの場合、PATH 解決を行う", () => {
    const expected = path.join("/opt/homebrew/bin", "tdsl");
    const result = resolveTdslBinary(
      "   ",
      { PATH: "/opt/homebrew/bin" },
      makeExists(new Set([expected])),
      "darwin",
    );
    assert.deepEqual(result, { path: expected });
  });

  // (g) PATH が未設定（undefined）でも notOnPath で安全に終わる
  it("PATH 環境変数が未設定でも notOnPath を返す", () => {
    const result = resolveTdslBinary(
      "",
      {},
      makeExists(new Set()),
      "linux",
    );
    assert.deepEqual(result, { error: "notOnPath" });
  });
});
