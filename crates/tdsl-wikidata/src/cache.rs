use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use tempfile::NamedTempFile;

use crate::client::{SearchResult, WikidataClient};
use crate::entity::WikidataEntity;
use crate::error::WikidataError;

/// キャッシュの動作を制御するオプション。
#[derive(Debug, Clone)]
pub struct CacheOptions {
    /// `true` の場合、キャッシュを無視して必ず API を呼び出す。
    pub no_cache: bool,
    /// キャッシュの有効期間。`Duration::ZERO` は `no_cache = true` と同等。
    pub ttl: Duration,
}

impl Default for CacheOptions {
    fn default() -> Self {
        Self {
            no_cache: false,
            ttl: Duration::from_secs(86400), // 24時間
        }
    }
}

/// `WikidataClient` をラップしてローカルファイルキャッシュを追加するデコレータ。
///
/// `get_entity` および `get_entity_by_sitelink` の結果を
/// `~/.cache/tdsl/` 以下の JSON ファイルにキャッシュする。
/// `search_entities` および `sparql_query` はキャッシュしない（動的な結果のため）。
pub struct CachedWikidataClient<C> {
    inner: C,
    cache_dir: PathBuf,
    opts: CacheOptions,
}

impl<C: WikidataClient> CachedWikidataClient<C> {
    /// 新しいキャッシュ付きクライアントを生成する。
    ///
    /// キャッシュディレクトリは `dirs::cache_dir()` が返す OS 標準のキャッシュ領域内に
    /// `tdsl` サブディレクトリを作成して使用する。
    /// `dirs::cache_dir()` が `None` を返す環境（一部CI等）では一時ディレクトリにフォールバックする。
    pub fn new(inner: C, opts: CacheOptions) -> Self {
        let cache_dir = dirs::cache_dir()
            .map(|d| d.join("tdsl"))
            .unwrap_or_else(|| std::env::temp_dir().join("tdsl_cache"));
        Self {
            inner,
            cache_dir,
            opts,
        }
    }

    /// テスト用: キャッシュディレクトリを明示指定して生成する。
    #[cfg(test)]
    pub fn with_cache_dir(inner: C, opts: CacheOptions, cache_dir: PathBuf) -> Self {
        Self {
            inner,
            cache_dir,
            opts,
        }
    }

    fn cache_path_for_entity(&self, qid: &str, langs: &[&str]) -> PathBuf {
        let langs_key = langs.join("-");
        self.cache_dir
            .join(format!("get_{}_{}.json", qid, langs_key))
    }

    fn cache_path_for_sitelink(&self, site: &str, title: &str, langs: &[&str]) -> PathBuf {
        let langs_key = langs.join("-");
        // ファイル名として使えない文字を置換してパスの安全性を確保
        let safe_title: String = title
            .chars()
            .map(|c| {
                if matches!(
                    c,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' '
                ) {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        self.cache_dir.join(format!(
            "sitelink_{}_{}_{}.json",
            site, safe_title, langs_key
        ))
    }

    /// キャッシュファイルが有効かどうかを判定する。
    /// `no_cache` が `true` または TTL が 0 の場合は常に `false`。
    fn is_cache_valid(&self, path: &Path) -> bool {
        if self.opts.no_cache || self.opts.ttl.is_zero() {
            return false;
        }
        let Ok(metadata) = std::fs::metadata(path) else {
            return false;
        };
        let Ok(modified) = metadata.modified() else {
            return false;
        };
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::MAX);
        age < self.opts.ttl
    }

    fn read_cache(&self, path: &Path) -> Option<WikidataEntity> {
        if !self.is_cache_valid(path) {
            return None;
        }
        let data = std::fs::read(path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    fn write_cache(&self, path: &Path, entity: &WikidataEntity) {
        if let Err(e) = std::fs::create_dir_all(&self.cache_dir) {
            eprintln!(
                "tdsl cache: ディレクトリ作成失敗 {}: {}",
                self.cache_dir.display(),
                e
            );
            return;
        }
        match serde_json::to_vec(entity) {
            Ok(data) => {
                // アトミック書き込み: 一時ファイルに書き込み後 rename(2) で切り替える。
                // 複数プロセスが同一ファイルに同時書き込みしても JSON が壊れない。
                match NamedTempFile::new_in(&self.cache_dir) {
                    Ok(mut tmp) => {
                        if let Err(e) = tmp.write_all(&data) {
                            eprintln!("tdsl cache: 一時ファイル書き込み失敗: {e}");
                            return;
                        }
                        if let Err(e) = tmp.persist(path) {
                            eprintln!("tdsl cache: persist 失敗 {}: {e}", path.display());
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "tdsl cache: 一時ファイル作成失敗 {}: {e}",
                            self.cache_dir.display()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("tdsl cache: シリアライズ失敗: {e}");
            }
        }
    }
}

#[async_trait]
impl<C: WikidataClient> WikidataClient for CachedWikidataClient<C> {
    async fn get_entity(&self, qid: &str, langs: &[&str]) -> Result<WikidataEntity, WikidataError> {
        let path = self.cache_path_for_entity(qid, langs);
        if let Some(entity) = self.read_cache(&path) {
            return Ok(entity);
        }
        let entity = self.inner.get_entity(qid, langs).await?;
        self.write_cache(&path, &entity);
        Ok(entity)
    }

    async fn get_entity_by_sitelink(
        &self,
        site: &str,
        title: &str,
        langs: &[&str],
    ) -> Result<WikidataEntity, WikidataError> {
        let path = self.cache_path_for_sitelink(site, title, langs);
        if let Some(entity) = self.read_cache(&path) {
            return Ok(entity);
        }
        let entity = self
            .inner
            .get_entity_by_sitelink(site, title, langs)
            .await?;
        self.write_cache(&path, &entity);
        Ok(entity)
    }

    /// 検索結果はキャッシュしない（動的な結果のため常に API を呼び出す）。
    async fn search_entities(
        &self,
        query: &str,
        lang: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, WikidataError> {
        self.inner.search_entities(query, lang, limit).await
    }

    /// SPARQL クエリはキャッシュしない（動的な結果のため常に API を呼び出す）。
    async fn sparql_query(&self, query: &str) -> Result<Vec<String>, WikidataError> {
        self.inner.sparql_query(query).await
    }
}

// ---------------------------------------------------------------------------
// キャッシュ管理 API（`tdsl cache` サブコマンド向け）
// ---------------------------------------------------------------------------

/// OS 標準のキャッシュディレクトリ内の `tdsl` サブディレクトリを返す。
///
/// `dirs::cache_dir()` が取得できない環境ではシステムの一時ディレクトリを使う。
pub fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("tdsl"))
        .unwrap_or_else(|| std::env::temp_dir().join("tdsl_cache"))
}

/// キャッシュの統計情報。
#[derive(Debug)]
pub struct CacheStatus {
    /// キャッシュディレクトリのパス。
    pub cache_dir: PathBuf,
    /// キャッシュファイルの総数。
    pub file_count: usize,
    /// キャッシュファイルの合計サイズ（バイト）。
    pub total_bytes: u64,
    /// 最も古いキャッシュエントリの最終更新時刻（ファイルが 1 件以上の場合）。
    pub oldest: Option<SystemTime>,
    /// 最も新しいキャッシュエントリの最終更新時刻（ファイルが 1 件以上の場合）。
    pub newest: Option<SystemTime>,
}

/// キャッシュの統計情報を収集して返す。
///
/// キャッシュディレクトリが存在しない場合はファイル数 0 の [`CacheStatus`] を返す。
pub fn cache_status(cache_dir: &Path) -> std::io::Result<CacheStatus> {
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;
    let mut oldest: Option<SystemTime> = None;
    let mut newest: Option<SystemTime> = None;

    if cache_dir.exists() {
        for entry in std::fs::read_dir(cache_dir)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_file() {
                file_count += 1;
                total_bytes += meta.len();
                if let Ok(modified) = meta.modified() {
                    oldest = Some(match oldest {
                        None => modified,
                        Some(prev) => prev.min(modified),
                    });
                    newest = Some(match newest {
                        None => modified,
                        Some(prev) => prev.max(modified),
                    });
                }
            }
        }
    }

    Ok(CacheStatus {
        cache_dir: cache_dir.to_path_buf(),
        file_count,
        total_bytes,
        oldest,
        newest,
    })
}

/// 直接パス構築で試す言語キー。`cache_path_for_entity` の `langs.join("-")` と
/// 同じ表記にすること（片方だけ変えると直接構築が外れ、毎回フォールバックの
/// 全走査に落ちる — 遅くなるだけで結果は正しいため、気づきにくい）。
const KNOWN_LANGS_KEYS: &[&str] = &["ja-en"];

/// 指定 QID のキャッシュ済みエンティティを読み出す（オフライン・ネットワーク不要）。
///
/// `<cache_dir>/get_<qid>_<langs>.json` のうち最初に見つかった有効なファイルを返す。
/// hover 表示用途のため **TTL は無視** する（古くても取得済み情報を見せる）。
/// キャッシュ未取得・ディレクトリ不在・パース失敗時は `None`。
pub fn read_cached_entity(cache_dir: &Path, qid: &str) -> Option<WikidataEntity> {
    // まず既知の言語キーでパスを直接組み立てる。ファイル名の規則は
    // `cache_path_for_entity` が決めており（`get_<qid>_<langs.join("-")>.json`）、
    // 実際に使われる言語セットは `["ja", "en"]` のみ。
    //
    // 以前は最初から `read_dir` で全エントリを線形走査しており、hover は
    // QID にカーソルを置くたびにこれを呼ぶため、キャッシュが数千ファイルに
    // 育つと応答が目に見えて遅くなっていた（#770）。
    for langs_key in KNOWN_LANGS_KEYS {
        let path = cache_dir.join(format!("get_{qid}_{langs_key}.json"));
        if let Ok(data) = std::fs::read(&path)
            && let Ok(entity) = serde_json::from_slice::<WikidataEntity>(&data)
        {
            return Some(entity);
        }
    }

    // フォールバックの全走査は残す。将来ほかの言語セットで取得された
    // キャッシュがあっても読めなくならないため（**削ると、規約から外れた
    // ファイルを黙って「無い」ことにしてしまう**）。
    let prefix = format!("get_{qid}_");
    let entries = std::fs::read_dir(cache_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&prefix)
            && name.ends_with(".json")
            && let Ok(data) = std::fs::read(entry.path())
            && let Ok(entity) = serde_json::from_slice::<WikidataEntity>(&data)
        {
            return Some(entity);
        }
    }
    None
}

/// キャッシュを削除する。
///
/// `older_than_days` が `Some(n)` の場合、最終更新から `n` 日以上経過したファイルのみを削除する。
/// `None` の場合はすべてのキャッシュファイルを削除する。
///
/// キャッシュディレクトリが存在しない場合は何もせずに `0` を返す（エラーにしない）。
///
/// 返り値: 削除したファイル数。
pub fn cache_clear(cache_dir: &Path, older_than_days: Option<u64>) -> std::io::Result<usize> {
    if !cache_dir.exists() {
        return Ok(0);
    }

    let threshold: Option<Duration> = older_than_days.map(|d| Duration::from_secs(d * 86400));
    let now = SystemTime::now();
    let mut deleted = 0usize;

    for entry in std::fs::read_dir(cache_dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if !meta.is_file() {
            continue;
        }

        let should_delete = match threshold {
            None => true,
            Some(min_age) => {
                let age = meta
                    .modified()
                    .ok()
                    .and_then(|m| now.duration_since(m).ok())
                    .unwrap_or(Duration::ZERO);
                age >= min_age
            }
        };

        if should_delete {
            std::fs::remove_file(entry.path())?;
            deleted += 1;
        }
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::WikidataEntity;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    /// API呼び出し回数を記録するモッククライアント
    struct MockClient {
        call_count: Arc<AtomicUsize>,
        entity: WikidataEntity,
    }

    impl MockClient {
        fn new(qid: &str) -> Self {
            Self {
                call_count: Arc::new(AtomicUsize::new(0)),
                entity: WikidataEntity {
                    id: qid.to_string(),
                    labels: HashMap::new(),
                    claims: HashMap::new(),
                },
            }
        }
    }

    #[async_trait]
    impl WikidataClient for MockClient {
        async fn get_entity(
            &self,
            _qid: &str,
            _langs: &[&str],
        ) -> Result<WikidataEntity, WikidataError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(self.entity.clone())
        }

        async fn get_entity_by_sitelink(
            &self,
            _site: &str,
            _title: &str,
            _langs: &[&str],
        ) -> Result<WikidataEntity, WikidataError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(self.entity.clone())
        }

        async fn search_entities(
            &self,
            _query: &str,
            _lang: &str,
            _limit: usize,
        ) -> Result<Vec<SearchResult>, WikidataError> {
            Ok(vec![])
        }

        async fn sparql_query(&self, _query: &str) -> Result<Vec<String>, WikidataError> {
            Ok(vec![])
        }
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().unwrap()
    }

    #[test]
    fn cache_hit_avoids_api_call() {
        let tmp = TempDir::new().unwrap();
        let mock = MockClient::new("Q42");
        let call_count = mock.call_count.clone();
        let client = CachedWikidataClient::with_cache_dir(
            mock,
            CacheOptions::default(),
            tmp.path().to_path_buf(),
        );

        rt().block_on(async {
            // 1回目: API呼び出しが発生してキャッシュに保存
            client.get_entity("Q42", &["ja", "en"]).await.unwrap();
            assert_eq!(call_count.load(Ordering::SeqCst), 1);

            // 2回目: キャッシュヒットでAPI呼び出しなし
            client.get_entity("Q42", &["ja", "en"]).await.unwrap();
            assert_eq!(call_count.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn no_cache_forces_api_call() {
        let tmp = TempDir::new().unwrap();
        let mock = MockClient::new("Q42");
        let call_count = mock.call_count.clone();
        let client = CachedWikidataClient::with_cache_dir(
            mock,
            CacheOptions {
                no_cache: true,
                ttl: Duration::from_secs(86400),
            },
            tmp.path().to_path_buf(),
        );

        rt().block_on(async {
            client.get_entity("Q42", &["ja"]).await.unwrap();
            client.get_entity("Q42", &["ja"]).await.unwrap();
            // no_cache=true なので毎回API呼び出し
            assert_eq!(call_count.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn expired_cache_triggers_api_call() {
        let tmp = TempDir::new().unwrap();
        let mock = MockClient::new("Q42");
        let call_count = mock.call_count.clone();

        // TTL 0 = キャッシュ無効（常にexpired扱い）
        let client = CachedWikidataClient::with_cache_dir(
            mock,
            CacheOptions {
                no_cache: false,
                ttl: Duration::ZERO,
            },
            tmp.path().to_path_buf(),
        );

        rt().block_on(async {
            client.get_entity("Q42", &["ja"]).await.unwrap();
            client.get_entity("Q42", &["ja"]).await.unwrap();
            // TTL=0 なので毎回API呼び出し
            assert_eq!(call_count.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn sitelink_cache_hit_avoids_api_call() {
        let tmp = TempDir::new().unwrap();
        let mock = MockClient::new("Q7209");
        let call_count = mock.call_count.clone();
        let client = CachedWikidataClient::with_cache_dir(
            mock,
            CacheOptions::default(),
            tmp.path().to_path_buf(),
        );

        rt().block_on(async {
            client
                .get_entity_by_sitelink("jawiki", "漢", &["ja"])
                .await
                .unwrap();
            assert_eq!(call_count.load(Ordering::SeqCst), 1);

            client
                .get_entity_by_sitelink("jawiki", "漢", &["ja"])
                .await
                .unwrap();
            // 2回目はキャッシュヒット
            assert_eq!(call_count.load(Ordering::SeqCst), 1);
        });
    }

    // ------------------------------------------------------------------
    // cache_status / cache_clear のテスト
    // ------------------------------------------------------------------

    #[test]
    fn cache_status_empty_dir_returns_zero_files() {
        let tmp = TempDir::new().unwrap();
        let status = cache_status(tmp.path()).unwrap();
        assert_eq!(status.file_count, 0);
        assert_eq!(status.total_bytes, 0);
        assert!(status.oldest.is_none());
        assert!(status.newest.is_none());
    }

    #[test]
    fn cache_status_nonexistent_dir_returns_zero_files() {
        let tmp = TempDir::new().unwrap();
        let nonexistent = tmp.path().join("no_such_dir");
        let status = cache_status(&nonexistent).unwrap();
        assert_eq!(status.file_count, 0);
        assert_eq!(status.total_bytes, 0);
    }

    #[test]
    fn cache_status_counts_files_and_bytes() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.json"), b"hello").unwrap();
        std::fs::write(tmp.path().join("b.json"), b"world!").unwrap();
        let status = cache_status(tmp.path()).unwrap();
        assert_eq!(status.file_count, 2);
        assert_eq!(status.total_bytes, 11); // 5 + 6
        assert!(status.oldest.is_some());
        assert!(status.newest.is_some());
    }

    #[test]
    fn cache_clear_nonexistent_dir_returns_zero() {
        let tmp = TempDir::new().unwrap();
        let nonexistent = tmp.path().join("no_such_dir");
        let deleted = cache_clear(&nonexistent, None).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn cache_clear_all_deletes_all_files() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.json"), b"x").unwrap();
        std::fs::write(tmp.path().join("b.json"), b"y").unwrap();
        let deleted = cache_clear(tmp.path(), None).unwrap();
        assert_eq!(deleted, 2);
        let status = cache_status(tmp.path()).unwrap();
        assert_eq!(status.file_count, 0);
    }

    #[test]
    fn cache_clear_older_than_does_not_delete_fresh_files() {
        let tmp = TempDir::new().unwrap();
        // 新しいファイルを作成（現時刻）
        std::fs::write(tmp.path().join("fresh.json"), b"x").unwrap();
        // 30日以上古いものだけ削除 → 新しいファイルは残る
        let deleted = cache_clear(tmp.path(), Some(30)).unwrap();
        assert_eq!(deleted, 0);
        let status = cache_status(tmp.path()).unwrap();
        assert_eq!(status.file_count, 1);
    }

    // ------------------------------------------------------------------
    // read_cached_entity のテスト
    // ------------------------------------------------------------------

    #[test]
    fn read_cached_entity_returns_entity_from_cache_file() {
        let tmp = TempDir::new().unwrap();
        let entity = WikidataEntity {
            id: "Q42".to_string(),
            labels: HashMap::new(),
            claims: HashMap::new(),
        };
        let data = serde_json::to_vec(&entity).unwrap();
        // キャッシュファイル名パターン: get_<QID>_<langs>.json
        std::fs::write(tmp.path().join("get_Q42_ja.json"), &data).unwrap();

        let result = super::read_cached_entity(tmp.path(), "Q42");
        assert!(
            result.is_some(),
            "キャッシュファイルが存在すれば Some を返す"
        );
        assert_eq!(result.unwrap().id, "Q42");
    }

    #[test]
    fn read_cached_entity_returns_none_when_not_cached() {
        let tmp = TempDir::new().unwrap();
        // Q42 のキャッシュファイルなし
        let result = super::read_cached_entity(tmp.path(), "Q42");
        assert!(result.is_none(), "キャッシュファイルが無ければ None を返す");
    }

    #[test]
    fn read_cached_entity_returns_none_for_nonexistent_cache_dir() {
        let tmp = TempDir::new().unwrap();
        let nonexistent = tmp.path().join("no_such_dir");
        let result = super::read_cached_entity(&nonexistent, "Q42");
        assert!(result.is_none(), "ディレクトリが存在しなければ None を返す");
    }

    /// 既知の言語キーのファイルは、ディレクトリ走査に頼らず直接引ける（#770）。
    ///
    /// 「走査していないこと」を直接は観測できないので、**走査では見つからない
    /// 状況を作って**確かめる: `read_dir` が失敗するようディレクトリを
    /// 読めなくする…のは移植性が低いため、ここでは代わりに
    /// 「大量のダミーがあっても正しい 1 件を返す」ことと、
    /// 命名規則から外れたファイルはフォールバックで拾えることを固定する。
    #[test]
    fn read_cached_entity_finds_known_langs_key_among_many_files() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..50 {
            std::fs::write(
                tmp.path().join(format!("get_Q{}_ja-en.json", 900000 + i)),
                r#"{"id":"other","labels":{},"claims":{}}"#,
            )
            .unwrap();
        }
        std::fs::write(
            tmp.path().join("get_Q42_ja-en.json"),
            r#"{"id":"Q42","labels":{},"claims":{}}"#,
        )
        .unwrap();

        let got = super::read_cached_entity(tmp.path(), "Q42").expect("見つかるべき");
        assert_eq!(got.id, "Q42");
    }

    /// 既知の言語キーから外れたファイル名でも、フォールバックの全走査で拾える。
    /// **直接構築だけにするとここが黙って「無い」ことになる**ため、
    /// フォールバックを削っていないことを固定する。
    #[test]
    fn read_cached_entity_falls_back_to_scan_for_unknown_langs_key() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("get_Q42_fr-de.json"),
            r#"{"id":"Q42","labels":{},"claims":{}}"#,
        )
        .unwrap();

        let got = super::read_cached_entity(tmp.path(), "Q42").expect("走査で拾えるべき");
        assert_eq!(got.id, "Q42");
    }

    #[test]
    fn read_cached_entity_ignores_unrelated_files() {
        let tmp = TempDir::new().unwrap();
        // Q7209 はあるが Q42 はない
        let entity = WikidataEntity {
            id: "Q7209".to_string(),
            labels: HashMap::new(),
            claims: HashMap::new(),
        };
        let data = serde_json::to_vec(&entity).unwrap();
        std::fs::write(tmp.path().join("get_Q7209_ja-en.json"), &data).unwrap();

        let result = super::read_cached_entity(tmp.path(), "Q42");
        assert!(
            result.is_none(),
            "プレフィクスが一致しないファイルは無視する"
        );
    }

    #[test]
    fn different_langs_use_different_cache_files() {
        let tmp = TempDir::new().unwrap();
        let mock = MockClient::new("Q42");
        let call_count = mock.call_count.clone();
        let client = CachedWikidataClient::with_cache_dir(
            mock,
            CacheOptions::default(),
            tmp.path().to_path_buf(),
        );

        rt().block_on(async {
            client.get_entity("Q42", &["ja"]).await.unwrap();
            client.get_entity("Q42", &["en"]).await.unwrap();
            // 言語が違うのでキャッシュキーが異なり、2回API呼び出し
            assert_eq!(call_count.load(Ordering::SeqCst), 2);
        });
    }
}
