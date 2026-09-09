#!/usr/bin/env bash
set -euo pipefail

# keroway/homebrew-tap の Formula/tdsl.rb を生成する。
# .github/workflows/release.yml の update-homebrew-formula ジョブから呼ばれる本体。
# scripts/test-homebrew-formula-generation.sh からもダミー値で同じロジックを呼び出し、
# 生成物の Ruby 構文とバージョン二重宣言の有無を検証する（#889）。
#
# 必須環境変数: VERSION, SHA_LINUX, SHA_LINUX_ARM, SHA_MACOS_X86, SHA_MACOS_ARM, OUTPUT_PATH

: "${VERSION:?VERSION is required}"
: "${SHA_LINUX:?SHA_LINUX is required}"
: "${SHA_LINUX_ARM:?SHA_LINUX_ARM is required}"
: "${SHA_MACOS_X86:?SHA_MACOS_X86 is required}"
: "${SHA_MACOS_ARM:?SHA_MACOS_ARM is required}"
: "${OUTPUT_PATH:?OUTPUT_PATH is required}"

# `version` は宣言しない。Homebrew は url のパスからバージョンを読むため、
# 併記すると `brew audit` が "version X is redundant with version scanned
# from URL" で落ち、tap 側の test-bot が失敗する（v2.1.0 で発生）。
# url にはタグを直書きする（keroway/homebrew-tap の CLAUDE.md の手順どおり）。
#
# 構造（caveats / 補完 / test）もここが正典。生成物で formula 全体を上書きするため、
# tap 側で公式を拡張したら必ずこの heredoc に同期しないと次回リリースで消える。
# （v2.1.0 で caveats / generate_completions / test 拡張が消えた事例。keroway/homebrew-tap#97）
# caveats 中の `tdsl` は非クォート heredoc のため \` でエスケープ必須。
TAG="${VERSION#v}"

cat > "$OUTPUT_PATH" <<FORMULA
class Tdsl < Formula
  desc "Timeline DSL compiler — text-based timelines with Wikidata import"
  homepage "https://github.com/keroway/timeline-dsl"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/keroway/timeline-dsl/releases/download/v${TAG}/tdsl-macos-aarch64.tar.gz"
      sha256 "${SHA_MACOS_ARM}"
    else
      url "https://github.com/keroway/timeline-dsl/releases/download/v${TAG}/tdsl-macos-x86_64.tar.gz"
      sha256 "${SHA_MACOS_X86}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/keroway/timeline-dsl/releases/download/v${TAG}/tdsl-linux-aarch64.tar.gz"
      sha256 "${SHA_LINUX_ARM}"
    else
      url "https://github.com/keroway/timeline-dsl/releases/download/v${TAG}/tdsl-linux-x86_64.tar.gz"
      sha256 "${SHA_LINUX}"
    end
  end

  def install
    bin.install "tdsl"
    generate_completions_from_executable(bin/"tdsl", "completions")
  end

  def caveats
    <<~EOS
      Bash, zsh, and fish completions for \`tdsl\` were generated, but Homebrew
      does not auto-link completions from external taps by default. Run this
      once to enable them:
        brew completions link
    EOS
  end

  test do
    assert_match "tdsl", shell_output("#{bin}/tdsl --version")
    (testpath/"test.tdsl").write <<~EOS
      timeline "test" {
        unit year;
        range 1..100;
      }
      lane "main" as main { kind dynasty; order 1; }
      span main 10..50 "test span" {};
    EOS
    assert_match "lanes", shell_output("#{bin}/tdsl build #{testpath}/test.tdsl --pretty")

    system bin/"tdsl", "render", testpath/"test.tdsl", "-o", testpath/"out.html"
    assert_path_exists testpath/"out.html"
    assert_match "<svg", (testpath/"out.html").read

    assert_path_exists bash_completion/"tdsl"
    assert_path_exists zsh_completion/"_tdsl"
    assert_path_exists fish_completion/"tdsl.fish"
  end
end
FORMULA
