use crate::engine::error::EngineError;
use crate::engine::models::BookMove;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// 象棋云库 (chessdb.cn) 客户端配置
#[derive(Debug, Clone)]
pub struct CloudBookConfig {
    pub base_url: String,
    pub timeout: Duration,
    pub enabled: bool,
}

impl Default for CloudBookConfig {
    fn default() -> Self {
        Self {
            base_url: "https://www.chessdb.cn/chessdb.php".to_string(),
            timeout: Duration::from_millis(2000),
            enabled: true,
        }
    }
}

struct CacheEntry {
    created_at: Instant,
    moves: Vec<BookMove>,
}

/// 象棋云库在线查询客户端，支持局面候选着法与评分获取、内存缓存与网络超时降级
#[derive(Clone)]
pub struct CloudBookClient {
    client: reqwest::Client,
    config: Arc<RwLock<CloudBookConfig>>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl CloudBookClient {
    pub fn new(config: CloudBookConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_default();

        Self {
            client,
            config: Arc::new(RwLock::new(config)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 设置云库是否启用
    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.enabled = enabled;
        }
    }

    /// 查询云库当前启用状态
    pub fn is_enabled(&self) -> bool {
        self.config.read().map(|c| c.enabled).unwrap_or(true)
    }

    /// 清空本地内存查询缓存
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// 解析云库 `queryall` 返回的原始字符串
    pub fn parse_response(text: &str) -> Vec<BookMove> {
        let trimmed = text.trim();
        if trimmed.is_empty()
            || trimmed == "unknown"
            || trimmed == "checkmate"
            || trimmed == "stalemate"
            || trimmed == "invalid board"
            || trimmed == "nobestmove"
        {
            return Vec::new();
        }

        let mut moves = Vec::new();
        for segment in trimmed.split('|') {
            let seg = segment.trim();
            if seg.is_empty() {
                continue;
            }

            let mut iccs = String::new();
            let mut score = 0i32;
            let mut win_rate = 50.0f64;
            let mut note_val = None;
            let mut is_egtb = false;

            for field in seg.split(',') {
                let f = field.trim();
                if let Some(mv) = f.strip_prefix("move:") {
                    iccs = mv.trim().to_string();
                } else if let Some(egtb) = f.strip_prefix("egtb:") {
                    iccs = egtb.trim().to_string();
                    is_egtb = true;
                } else if let Some(search) = f.strip_prefix("search:") {
                    iccs = search.trim().to_string();
                } else if let Some(sc) = f.strip_prefix("score:") {
                    score = sc.trim().parse::<i32>().unwrap_or(0);
                } else if let Some(wr) = f.strip_prefix("winrate:") {
                    win_rate = wr.trim().parse::<f64>().unwrap_or(50.0);
                } else if let Some(nt) = f.strip_prefix("note:") {
                    let cleaned = nt.trim();
                    if !cleaned.is_empty() {
                        note_val = Some(cleaned.to_string());
                    }
                }
            }

            if !iccs.is_empty() {
                let source = if is_egtb {
                    "象棋云库 (残局)".to_string()
                } else {
                    "象棋云库".to_string()
                };

                moves.push(BookMove {
                    iccs,
                    notation: None,
                    score,
                    win_count: 0,
                    draw_count: 0,
                    lose_count: 0,
                    win_rate,
                    note: note_val,
                    source,
                });
            }
        }

        moves
    }

    /// 查询指定 FEN 局面的云库着法列表。网络不可用或超时时静默返回空列表
    pub async fn query(&self, fen: &str) -> Result<Vec<BookMove>, EngineError> {
        if !self.is_enabled() {
            return Ok(Vec::new());
        }

        // 1. 检查内存缓存（10 分钟 TTL，最多保留 512 条）
        let normalized_fen = fen.trim();
        if let Ok(cache) = self.cache.read()
            && let Some(entry) = cache.get(normalized_fen)
            && entry.created_at.elapsed() < Duration::from_secs(600)
        {
            return Ok(entry.moves.clone());
        }

        // 2. 发起 HTTP GET 请求
        let (base_url, timeout) = self
            .config
            .read()
            .map(|c| (c.base_url.clone(), c.timeout))
            .unwrap_or_else(|_| {
                let def = CloudBookConfig::default();
                (def.base_url, def.timeout)
            });

        let res = self
            .client
            .get(&base_url)
            .query(&[
                ("action", "queryall"),
                ("board", normalized_fen),
                ("showall", "1"),
            ])
            .timeout(timeout)
            .send()
            .await;

        let moves = match res {
            Ok(resp) if resp.status().is_success() => {
                let body = resp.text().await.unwrap_or_default();
                Self::parse_response(&body)
            }
            _ => {
                // 网络异常、超时或非 200 响应时优雅降级为未命中
                Vec::new()
            }
        };

        // 3. 写入内存缓存
        if let Ok(mut cache) = self.cache.write() {
            if cache.len() >= 512 {
                cache.clear();
            }
            cache.insert(
                normalized_fen.to_string(),
                CacheEntry {
                    created_at: Instant::now(),
                    moves: moves.clone(),
                },
            );
        }

        Ok(moves)
    }
}

impl Default for CloudBookClient {
    fn default() -> Self {
        Self::new(CloudBookConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_and_special_responses() {
        assert!(CloudBookClient::parse_response("").is_empty());
        assert!(CloudBookClient::parse_response("unknown").is_empty());
        assert!(CloudBookClient::parse_response("invalid board").is_empty());
        assert!(CloudBookClient::parse_response("checkmate").is_empty());
        assert!(CloudBookClient::parse_response("stalemate").is_empty());
    }

    #[test]
    fn test_parse_valid_cloud_moves() {
        let sample = "move:c0e2,score:1,rank:2,note:! (44-04),winrate:50.08|move:b2e2,score:-15,rank:0,note:? (42-01),winrate:48.86";
        let moves = CloudBookClient::parse_response(sample);

        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0].iccs, "c0e2");
        assert_eq!(moves[0].score, 1);
        assert_eq!(moves[0].win_rate, 50.08);
        assert_eq!(moves[0].note.as_deref(), Some("! (44-04)"));
        assert_eq!(moves[0].source, "象棋云库");

        assert_eq!(moves[1].iccs, "b2e2");
        assert_eq!(moves[1].score, -15);
        assert_eq!(moves[1].win_rate, 48.86);
        assert_eq!(moves[1].note.as_deref(), Some("? (42-01)"));
    }

    #[test]
    fn test_parse_egtb_move() {
        let sample = "egtb:d0e1,score:250,rank:2,note:W (M-15),winrate:99.90";
        let moves = CloudBookClient::parse_response(sample);

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].iccs, "d0e1");
        assert_eq!(moves[0].score, 250);
        assert_eq!(moves[0].source, "象棋云库 (残局)");
    }
}
