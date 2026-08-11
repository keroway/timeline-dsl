import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { buildErrorHtml, buildPreviewHtml } from "./previewHtml.js";

// `PreviewController` は `vscode` API に依存するためここでは扱わない。
// HTML 組み立ては純関数なので、そこだけを固定する（#754）。

describe("buildPreviewHtml", () => {
  it("SVG を本文へ埋め込む", () => {
    const html = buildPreviewHtml("<svg><rect/></svg>", "vscode-resource:");
    assert.ok(html.includes("<svg><rect/></svg>"));
  });

  // Webview の CSP は絞る。プレビューは静的表示に徹し、
  // インタラクティブ機能は WebUI の役割とする。
  it("script を許可しない CSP を含む", () => {
    const html = buildPreviewHtml("<svg/>", "vscode-resource:");
    assert.ok(html.includes("default-src 'none'"));
    assert.ok(!html.includes("script-src"));
  });

  it("Webview の cspSource を img-src に反映する", () => {
    const html = buildPreviewHtml("<svg/>", "https://example.test");
    assert.ok(html.includes("img-src https://example.test"));
  });
});

describe("buildErrorHtml", () => {
  // 失敗を無音にしない。理由をそのまま出す（#754 の受け入れ条件）。
  it("エラーメッセージを表示する", () => {
    const html = buildErrorHtml("Error: something went wrong");
    assert.ok(html.includes("Error: something went wrong"));
  });

  // stderr には利用者の DSL 断片が含まれうる。そのまま埋めると
  // Webview の DOM を壊す（`<` が閉じないタグとして解釈される）。
  it("HTML 特殊文字をエスケープする", () => {
    const html = buildErrorHtml('unexpected <span> & "quote"');
    assert.ok(html.includes("&lt;span&gt;"));
    assert.ok(html.includes("&amp;"));
    assert.ok(!html.includes("<span>"));
  });
});
