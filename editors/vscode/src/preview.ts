import * as vscode from "vscode";
import { buildErrorHtml, buildPreviewHtml, renderSvg } from "./previewHtml";

/**
 * `.tdsl` の SVG プレビューを Webview で表示する。
 *
 * ## `--watch` を使わない理由（#754）
 *
 * `tdsl render --watch` は CLI 側で常駐しファイル変更を監視するが、拡張から
 * 使うと**子プロセスのライフサイクル管理が拡張の責務になる**（ウィンドウを
 * 閉じたとき、ワークスペースを切り替えたとき、拡張が再読み込みされたときに
 * 確実に止める必要がある）。取りこぼすとプロセスが残り続ける。
 *
 * 代わりに拡張側の保存/変更イベントで単発の `tdsl render` を回す。
 * 1 回の実行は短く、プロセスは必ず終了する。
 */

/** レンダリングの debounce（ミリ秒）。入力のたびに子プロセスを起動しないため。 */
const RENDER_DEBOUNCE_MS = 300;

/**
 * プレビューパネルを開き、対象ファイルの変更に追従させる。
 *
 * 同時に開くパネルは 1 つだけにする（対象ファイルを切り替えたら作り直す）。
 * 複数開けるようにすると、どのパネルがどのファイルのものか分からなくなる。
 */
export class PreviewController {
  private panel: vscode.WebviewPanel | undefined;
  private targetPath: string | undefined;
  private timer: NodeJS.Timeout | undefined;
  private readonly disposables: vscode.Disposable[] = [];

  constructor(private readonly binaryPath: string) {}

  async open(document: vscode.TextDocument): Promise<void> {
    this.targetPath = document.uri.fsPath;

    if (!this.panel) {
      this.panel = vscode.window.createWebviewPanel(
        "timelineDslPreview",
        "Timeline DSL Preview",
        vscode.ViewColumn.Beside,
        { enableScripts: false, retainContextWhenHidden: true },
      );
      this.panel.onDidDispose(() => {
        this.panel = undefined;
        this.targetPath = undefined;
        this.clearTimer();
      });
    } else {
      this.panel.reveal(vscode.ViewColumn.Beside, true);
    }

    await this.refresh();
  }

  /** 対象ファイルの変更を受けて再描画する（debounce 付き）。 */
  scheduleRefresh(changed: vscode.TextDocument): void {
    if (!this.panel || changed.uri.fsPath !== this.targetPath) {
      return;
    }
    this.clearTimer();
    this.timer = setTimeout(() => {
      void this.refresh();
    }, RENDER_DEBOUNCE_MS);
  }

  private async refresh(): Promise<void> {
    const panel = this.panel;
    const target = this.targetPath;
    if (!panel || !target) {
      return;
    }
    const result = await renderSvg(this.binaryPath, target);
    // await の間にパネルが閉じられている可能性がある。
    if (!this.panel) {
      return;
    }
    panel.webview.html = result.ok
      ? buildPreviewHtml(result.svg, panel.webview.cspSource)
      : buildErrorHtml(result.message);
  }

  private clearTimer(): void {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = undefined;
    }
  }

  dispose(): void {
    this.clearTimer();
    this.panel?.dispose();
    for (const d of this.disposables) {
      d.dispose();
    }
  }
}
