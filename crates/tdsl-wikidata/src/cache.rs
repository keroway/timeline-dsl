use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;

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
                if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ') {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        self.cache_dir
            .join(format!("sitelink_{}_{}_{}.json", site, safe_title, langs_key))
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
                if let Err(e) = std::fs::write(path, &data) {
                    eprintln!("tdsl cache: 書き込み失敗 {}: {}", path.display(), e);
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
        let entity = self.inner.get_entity_by_sitelink(site, title, langs).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::WikidataEntity;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
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
