pub(crate) mod build;
pub(crate) mod cache;
pub(crate) mod check;
pub(crate) mod decompile;
pub(crate) mod export_csv;
pub(crate) mod fetch;
pub(crate) mod fmt;
pub(crate) mod init;
pub(crate) mod lint;
pub(crate) mod lsp;
pub(crate) mod render;
pub(crate) mod scaffold;

/// `check` / `lint` / `fmt` の入力パス列を、実際に処理する `.tdsl` ファイルへ展開する。
///
/// - ファイルパスはそのまま採用する（拡張子は問わない。明示的に指定された
///   ものを勝手に無視しない）
/// - ディレクトリは再帰的に走査し、`.tdsl` だけを拾う
/// - 見つからなければ**エラーにする**。0 件を成功で返すと、パスの打ち間違いが
///   「問題なし」として通る（#750）
///
/// 走査順はパス名でソートする。**ファイルシステムの列挙順に依存させない** —
/// 診断の出力順が実行ごとに変わると、CI のログ差分が読めなくなる。
///
/// 新規依存を足さずに `std` だけで実装する（walkdir / glob は入れない）。
/// glob 展開はシェルに任せる方針（issue #750）。
pub(crate) fn resolve_tdsl_inputs(
    paths: &[std::path::PathBuf],
) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = Vec::new();
    for path in paths {
        if path.is_dir() {
            collect_tdsl_files(path, &mut out)?;
        } else if path.exists() {
            out.push(path.clone());
        } else {
            return Err(format!(
                "Failed to read {}: no such file or directory",
                path.display()
            ));
        }
    }

    if out.is_empty() {
        // ディレクトリを渡して 1 件も無い場合。silent success にしない。
        let joined = paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("No .tdsl files found under: {joined}"));
    }

    out.sort();
    out.dedup();
    Ok(out)
}

fn collect_tdsl_files(
    dir: &std::path::Path,
    out: &mut Vec<std::path::PathBuf>,
) -> Result<(), String> {
    // 実体パスを訪問済みとして記録し、シンボリックリンク経由で同じ
    // ディレクトリへ戻ってきたら打ち切る。**これが無いと、親を指す
    // シンボリックリンクがあるだけで無限に潜る**（実際に
    // `sub/loop/sub/loop/...` と再帰し続けることを確認した）。
    let mut visited = std::collections::HashSet::new();
    collect_tdsl_files_inner(dir, out, &mut visited)
}

fn collect_tdsl_files_inner(
    dir: &std::path::Path,
    out: &mut Vec<std::path::PathBuf>,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<(), String> {
    // canonicalize でシンボリックリンクを解決した実体を鍵にする。
    // 失敗した場合（権限など）は元のパスで代用し、走査自体は続ける。
    let key = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if !visited.insert(key) {
        // 既に見たディレクトリ。同じ実体を二度処理しない。
        return Ok(());
    }

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_tdsl_files_inner(&path, out, visited)?;
        } else if path.extension().is_some_and(|ext| ext == "tdsl") {
            out.push(path);
        }
    }
    Ok(())
}

/// 複数ファイルを 1 つずつ処理し、**1 件でも失敗すれば非ゼロ終了する**。
///
/// 最初の失敗で打ち切らない。CI では「どのファイルが落ちたか」を一度に
/// 知りたいため、全件処理してから結果をまとめる。
pub(crate) fn run_over_inputs(
    inputs: &[std::path::PathBuf],
    mut run: impl FnMut(&std::path::Path) -> Result<(), String>,
) -> Result<(), String> {
    let multi = inputs.len() > 1;
    let mut failed = Vec::new();
    for path in inputs {
        if multi {
            // どのファイルの診断かが分かるよう見出しを出す。
            eprintln!("=== {} ===", path.display());
        }
        if let Err(e) = run(path) {
            if !e.is_empty() {
                eprintln!("{e}");
            }
            failed.push(path.display().to_string());
        }
    }

    if failed.is_empty() {
        return Ok(());
    }
    // 個々のエラーは既に出力済みなので、ここでは要約だけを返す。
    Err(format!(
        "{} of {} file(s) failed: {}",
        failed.len(),
        inputs.len(),
        failed.join(", ")
    ))
}

/// ファイルのソーステキストを読み込む。
pub(crate) fn read_source(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))
}

/// カンマ区切りの言語コードをパースする。空入力時は `["en"]` を返す。
pub(crate) fn parse_langs(lang: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in lang.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lowered = trimmed.to_ascii_lowercase();
        if !out.iter().any(|x| x == &lowered) {
            out.push(lowered);
        }
    }
    if out.is_empty() {
        out.push("en".to_string());
    }
    out
}

/// DSL 文字列リテラル中でエスケープが必要な文字をエスケープする。
pub(crate) fn escape_tdsl_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

/// `seen` に登録済みなら suffix `_N` を付けてユニークな識別子を生成する。
pub(crate) fn make_unique_alias(
    seed: &str,
    seen: &mut std::collections::HashSet<String>,
) -> String {
    let mut alias = if seed.is_empty() {
        "item".to_string()
    } else {
        seed.to_string()
    };
    if !alias
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic() || c == '_')
        .unwrap_or(false)
    {
        alias = format!("_{}", alias);
    }

    if seen.insert(alias.clone()) {
        return alias;
    }
    let mut i = 2usize;
    loop {
        let cand = format!("{alias}_{i}");
        if seen.insert(cand.clone()) {
            return cand;
        }
        i += 1;
    }
}

/// `export-csv` の `tags` 列（`|` 区切り）へタグ配列をエンコードする。
///
/// タグ内の `\`・`|` は常にエスケープし、各タグの先頭・末尾が空白文字の場合は
/// `\s`（半角スペース）/ `\t` / `\n` / `\r` の記号エスケープで保護する。これは
/// `import-csv` 側の CSV reader が `Trim::All`（フィールド全体の前後空白除去）を
/// 有効にしているため、エスケープしない生の空白文字がタグ列全体の先頭・末尾に
/// 来ると無警告で失われるのを防ぐため（#885）。内側の空白はそのまま出力してよい。
pub(crate) fn encode_csv_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|t| escape_csv_tag(t))
        .collect::<Vec<_>>()
        .join("|")
}

fn escape_csv_tag(tag: &str) -> String {
    let chars: Vec<char> = tag.chars().collect();
    let last_idx = chars.len().saturating_sub(1);
    let mut out = String::with_capacity(tag.len());
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            _ if (i == 0 || i == last_idx) && c.is_whitespace() => {
                if let Some(code) = whitespace_escape_code(c) {
                    out.push('\\');
                    out.push(code);
                } else {
                    // 記号化できない空白文字（改行以外の稀な Unicode 空白等）はそのまま出力する。
                    // タグの内側ではないため理論上 Trim::All の影響を受けうるが、
                    // ASCII 空白 4 種以外は実運用で発生しないため許容する。
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn whitespace_escape_code(c: char) -> Option<char> {
    match c {
        ' ' => Some('s'),
        '\t' => Some('t'),
        '\n' => Some('n'),
        '\r' => Some('r'),
        _ => None,
    }
}

fn whitespace_from_escape_code(c: char) -> Option<char> {
    match c {
        's' => Some(' '),
        't' => Some('\t'),
        'n' => Some('\n'),
        'r' => Some('\r'),
        _ => None,
    }
}

/// `export-csv` / 手書き CSV の `tags` 列（`|` 区切り）をタグ配列へデコードする。
///
/// `encode_csv_tags` と対称。`\\`（リテラル `\`）、`\|`（リテラル `|`）、
/// `\s`/`\t`/`\n`/`\r`（境界空白の記号エスケープ）以外の `\<char>` は
/// 未知のエスケープとしてエラーにする（silent fallback 禁止、#885）。
pub(crate) fn decode_csv_tags(raw: &str) -> Result<Vec<String>, String> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }

    let mut tags = Vec::new();
    let mut current = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(next @ ('\\' | '|')) => current.push(next),
                Some(next) => match whitespace_from_escape_code(next) {
                    Some(ws) => current.push(ws),
                    None => {
                        return Err(format!(
                            "invalid tag escape `\\{next}` (expected `\\\\`, `\\|`, `\\s`, `\\t`, `\\n`, or `\\r`)"
                        ));
                    }
                },
                None => {
                    return Err("tag ends with a dangling `\\` escape".to_string());
                }
            },
            '|' => tags.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    tags.push(current);
    Ok(tags)
}

/// ASCII 英数字・スペース・ハイフン・アンダースコアのみを抽出して slug を生成する。
pub(crate) fn slug_ascii(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' || c == '-' || c == '_' {
                Some('_')
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── 複数ファイル / ディレクトリ入力（#750）─────────────────────────

    /// テスト用の一時ディレクトリ。`tempfile` は dev-dependency に無いので
    /// 自前で作って `Drop` で消す（新規依存を足さないため）。
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            // プロセス ID とタグで衝突を避ける。テストは並列に走る。
            let dir = std::env::temp_dir().join(format!("tdsl-test-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("create temp dir");
            Self(dir)
        }

        fn write(&self, rel: &str, body: &str) -> std::path::PathBuf {
            let path = self.0.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create parent");
            }
            std::fs::write(&path, body).expect("write file");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// ディレクトリを渡すと `.tdsl` を**再帰的に**拾う。
    #[test]
    fn resolve_inputs_walks_directories_recursively() {
        let tmp = TempDir::new("walk");
        tmp.write("a.tdsl", "");
        tmp.write("sub/b.tdsl", "");
        tmp.write("sub/deep/c.tdsl", "");
        // .tdsl 以外は拾わない
        tmp.write("readme.md", "");
        tmp.write("sub/notes.txt", "");

        let got = resolve_tdsl_inputs(std::slice::from_ref(&tmp.0)).expect("should resolve");
        let names: Vec<String> = got
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["a.tdsl", "b.tdsl", "c.tdsl"], "got: {got:?}");
    }

    /// **走査順はソートする。** ファイルシステムの列挙順に依存すると、
    /// 診断の出力順が実行ごとに変わって CI のログ差分が読めなくなる。
    #[test]
    fn resolve_inputs_is_sorted_and_deduped() {
        let tmp = TempDir::new("sort");
        let a = tmp.write("zzz.tdsl", "");
        tmp.write("aaa.tdsl", "");

        // 同じファイルを 2 回渡しても 1 件になる
        let got = resolve_tdsl_inputs(&[tmp.0.clone(), a.clone(), a]).expect("should resolve");
        assert_eq!(got.len(), 2, "重複が除かれていない: {got:?}");
        assert!(got[0] < got[1], "ソートされていない: {got:?}");
    }

    /// 明示的に指定されたファイルは拡張子を問わず採用する
    /// （利用者が指定したものを勝手に無視しない）。
    #[test]
    fn resolve_inputs_accepts_explicit_file_regardless_of_extension() {
        let tmp = TempDir::new("explicit");
        let odd = tmp.write("timeline.txt", "");
        let got = resolve_tdsl_inputs(std::slice::from_ref(&odd)).expect("should resolve");
        assert_eq!(got, vec![odd]);
    }

    /// **親を指すシンボリックリンクで無限再帰しない。**
    /// 対策が無いと `sub/loop/sub/loop/...` と潜り続ける（実際に確認した）。
    #[test]
    fn resolve_inputs_does_not_loop_on_directory_symlinks() {
        let tmp = TempDir::new("symlink");
        tmp.write("a.tdsl", "");
        let sub = tmp.0.join("sub");
        std::fs::create_dir_all(&sub).expect("create sub");

        // 親を指すシンボリックリンク。Windows では作成できないことがあるので、
        // 失敗したらこのテストはスキップする（対策の要否は POSIX 側で確認する）。
        #[cfg(unix)]
        {
            if std::os::unix::fs::symlink(&tmp.0, sub.join("loop")).is_err() {
                return;
            }
            let got = resolve_tdsl_inputs(std::slice::from_ref(&tmp.0)).expect("should resolve");
            assert_eq!(got.len(), 1, "同じファイルを重複して拾っている: {got:?}");
        }
    }

    /// **1 件も見つからなければエラー。** 0 件を成功で返すと、パスの
    /// 打ち間違いが「問題なし」として通る。
    #[test]
    fn resolve_inputs_errors_when_nothing_found() {
        let tmp = TempDir::new("empty");
        tmp.write("readme.md", "");
        let err = resolve_tdsl_inputs(std::slice::from_ref(&tmp.0)).expect_err("空なら失敗すべき");
        assert!(err.contains("No .tdsl files found"), "got: {err}");
    }

    #[test]
    fn resolve_inputs_errors_on_missing_path() {
        let missing = std::env::temp_dir().join("tdsl-does-not-exist-12345.tdsl");
        let err = resolve_tdsl_inputs(&[missing]).expect_err("存在しないなら失敗すべき");
        assert!(err.contains("no such file"), "got: {err}");
    }

    /// **最初の失敗で打ち切らない。** CI では「どのファイルが落ちたか」を
    /// 一度に知りたいため、全件処理してから結果をまとめる。
    #[test]
    fn run_over_inputs_processes_all_and_reports_every_failure() {
        let paths: Vec<std::path::PathBuf> = ["a", "b", "c"]
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        let mut seen = Vec::new();
        let err = run_over_inputs(&paths, |p| {
            seen.push(p.display().to_string());
            if p.ends_with("a") || p.ends_with("c") {
                Err(String::new())
            } else {
                Ok(())
            }
        })
        .expect_err("失敗があれば Err");

        assert_eq!(seen, vec!["a", "b", "c"], "途中で打ち切っている");
        assert!(err.contains("2 of 3"), "got: {err}");
        assert!(err.contains('a') && err.contains('c'), "got: {err}");
    }

    #[test]
    fn run_over_inputs_is_ok_when_all_succeed() {
        let paths: Vec<std::path::PathBuf> =
            ["a", "b"].iter().map(std::path::PathBuf::from).collect();
        assert!(run_over_inputs(&paths, |_| Ok(())).is_ok());
    }

    #[test]
    fn parse_langs_dedup_and_trim() {
        let langs = parse_langs(" ja, en,ja, ,ZH ");
        assert_eq!(langs, vec!["ja", "en", "zh"]);
    }

    #[test]
    fn parse_langs_empty_defaults_to_en() {
        let langs = parse_langs("");
        assert_eq!(langs, vec!["en"]);
    }

    #[test]
    fn parse_langs_lowercases_and_deduplicates() {
        let langs = parse_langs("JA,en,JA");
        assert_eq!(langs, vec!["ja", "en"]);
    }

    #[test]
    fn escape_tdsl_string_escapes_backslash_and_quote() {
        assert_eq!(escape_tdsl_string(r#"a"b\c"#), r#"a\"b\\c"#);
    }

    #[test]
    fn make_unique_alias_deduplicates() {
        let mut seen = std::collections::HashSet::new();
        let a = make_unique_alias("foo", &mut seen);
        let b = make_unique_alias("foo", &mut seen);
        assert_eq!(a, "foo");
        assert_eq!(b, "foo_2");
    }

    #[test]
    fn slug_ascii_filters_non_ascii() {
        assert_eq!(slug_ascii("Hello World"), "hello_world");
        assert_eq!(slug_ascii("漢"), "");
        assert_eq!(slug_ascii("abc-123"), "abc_123");
    }

    // ─── CSV tags エンコード/デコード（#885: 可逆エスケープ）─────────────

    #[test]
    fn encode_csv_tags_plain_tags_unchanged() {
        // 区切り文字・境界空白を含まない従来どおりのタグは以前と同じ `|` 結合になる。
        assert_eq!(
            encode_csv_tags(&["war".to_string(), "global".to_string()]),
            "war|global"
        );
    }

    #[test]
    fn encode_then_decode_round_trips_tags_with_separators_and_padding() {
        // issue #885 記載のケース: 区切り文字自体や前後空白を含むタグが、
        // export→import の往復で無警告に別タグへ分割されないこと。
        let tags = vec!["a|b".to_string(), "c,d".to_string(), " padded ".to_string()];
        let encoded = encode_csv_tags(&tags);
        let decoded = decode_csv_tags(&encoded).expect("decode should succeed");
        assert_eq!(decoded, tags);
    }

    #[test]
    fn encode_csv_tags_escapes_boundary_whitespace_so_csv_trim_is_safe() {
        // `import-csv` の CSV reader は Trim::All でフィールド全体の前後空白を
        // 除去するため、エンコード後の文字列の絶対先頭・末尾が空白文字であっては
        // ならない（#885）。
        let encoded = encode_csv_tags(&[" padded ".to_string()]);
        assert!(!encoded.starts_with(' ') && !encoded.ends_with(' '));
        assert_eq!(encoded, "\\spadded\\s");
    }

    #[test]
    fn decode_csv_tags_empty_string_is_zero_tags() {
        assert_eq!(decode_csv_tags("").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn decode_csv_tags_rejects_unknown_escape() {
        let err = decode_csv_tags(r"a\qb").unwrap_err();
        assert!(err.contains("invalid tag escape"), "got: {err}");
    }

    #[test]
    fn decode_csv_tags_rejects_dangling_backslash() {
        let err = decode_csv_tags(r"a\").unwrap_err();
        assert!(err.contains("dangling"), "got: {err}");
    }
}
