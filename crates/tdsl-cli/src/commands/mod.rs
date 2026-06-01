pub(crate) mod build;
pub(crate) mod cache;
pub(crate) mod check;
pub(crate) mod decompile;
pub(crate) mod fetch;
pub(crate) mod fmt;
pub(crate) mod init;
pub(crate) mod lint;
pub(crate) mod lsp;
pub(crate) mod render;
pub(crate) mod scaffold;

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
