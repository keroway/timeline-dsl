use crate::CacheAction;

/// Wikidata キャッシュを管理する。
pub(crate) fn cmd_cache(action: CacheAction) -> Result<(), String> {
    let cache_dir = tdsl_wikidata::default_cache_dir();

    match action {
        CacheAction::Status => {
            let status = tdsl_wikidata::cache_status(&cache_dir).map_err(|e| e.to_string())?;

            println!("Cache directory: {}", status.cache_dir.display());

            if !status.cache_dir.exists() {
                println!("Status: cache directory does not exist (no cached entries)");
                return Ok(());
            }

            println!("Files:      {}", status.file_count);
            println!("Total size: {}", human_bytes(status.total_bytes));

            if let Some(oldest) = status.oldest {
                println!("Oldest:     {}", format_system_time(oldest));
            }
            if let Some(newest) = status.newest {
                println!("Newest:     {}", format_system_time(newest));
            }

            if status.file_count == 0 {
                println!("Status: cache is empty");
            }
        }

        CacheAction::Clear { older_than } => {
            let deleted =
                tdsl_wikidata::cache_clear(&cache_dir, older_than).map_err(|e| e.to_string())?;

            match older_than {
                Some(days) => println!("Deleted {deleted} cache file(s) older than {days} day(s)."),
                None => println!("Deleted {deleted} cache file(s)."),
            }
        }
    }

    Ok(())
}

/// ファイルサイズを人間が読みやすい形式に変換する。
fn human_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// `SystemTime` を `YYYY-MM-DD HH:MM:SS UTC` 形式の文字列に変換する。
fn format_system_time(t: std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // 簡易 UTC 変換（外部クレートなし）
    let s = secs.rem_euclid(60);
    let m = (secs / 60).rem_euclid(60);
    let h = (secs / 3600).rem_euclid(24);
    let days = secs / 86400;
    // https://howardhinnant.github.io/date_algorithms.html#civil_from_days
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02} UTC")
}
