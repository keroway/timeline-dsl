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
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_tdsl_files(&path, out)?;
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
}
