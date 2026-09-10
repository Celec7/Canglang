use crate::core::board::BoardState;
use crate::core::hash::ZobristHasher;
use crate::engine::book::bh_book::{BhOpenBook, IOpeningBook};
use crate::engine::book::cloud_book::CloudBookClient;
use crate::engine::error::EngineError;
use crate::engine::models::BookMove;
use std::cmp::Ordering;

/// 云端与本地开局库协同查询策略
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum CloudBookMode {
    /// 协同模式：本地优先，脱谱或无本地库时自动查询云库（推荐）
    #[default]
    Hybrid,
    /// 纯云库模式：仅查询象棋云库
    CloudOnly,
    /// 纯本地模式：仅查询本地加载的 .bh 库（离线）
    LocalOnly,
    /// 合并模式：同时查询本地库与云库，合并去重后按评分排序
    Merge,
}

/// 开局库综合管理与查询服务，支持并发加载多个本地 `.bh` 开局库与在线象棋云库 (chessdb.cn)
pub struct OpeningBookService {
    books: Vec<Box<dyn IOpeningBook>>,
    cloud: CloudBookClient,
    mode: CloudBookMode,
}

impl OpeningBookService {
    pub fn new() -> Self {
        Self {
            books: Vec::new(),
            cloud: CloudBookClient::default(),
            mode: CloudBookMode::Hybrid,
        }
    }

    /// 设置云库查询策略
    pub fn set_cloud_mode(&mut self, mode: CloudBookMode) {
        self.mode = mode;
    }

    /// 获取当前云库查询策略
    pub fn cloud_mode(&self) -> CloudBookMode {
        self.mode
    }

    /// 设置云库是否启用
    pub fn set_cloud_enabled(&mut self, enabled: bool) {
        self.cloud.set_enabled(enabled);
    }

    /// 云库当前是否处于启用状态
    pub fn is_cloud_enabled(&self) -> bool {
        self.cloud.is_enabled()
    }

    /// 获取云库底层客户端引用
    pub fn cloud(&self) -> &CloudBookClient {
        &self.cloud
    }

    /// 加载指定路径的 `.bh` 开局库。若已加载则忽略
    pub fn load_book(&mut self, path: &str) -> Result<(), EngineError> {
        if path.trim().is_empty() {
            return Err(EngineError::InvalidConfig(
                "book path cannot be empty".to_string(),
            ));
        }

        let lower = path.to_lowercase();
        if self.books.iter().any(|b| b.path().to_lowercase() == lower) {
            return Ok(());
        }

        let book = BhOpenBook::open(path)?;
        self.books.push(Box::new(book));
        Ok(())
    }

    /// 注册开局库实例（支持注入内存或自定义 IOpeningBook 实例用于单测）
    pub fn register_book(&mut self, book: Box<dyn IOpeningBook>) {
        self.books.push(book);
    }

    /// 卸载并释放指定路径的开局库
    pub fn unload_book(&mut self, path: &str) {
        let lower = path.to_lowercase();
        if let Some(pos) = self
            .books
            .iter()
            .position(|b| b.path().to_lowercase() == lower)
        {
            self.books.remove(pos);
        }
    }

    /// 卸载所有开局库
    pub fn clear_books(&mut self) {
        self.books.clear();
    }

    /// 查询本地 `.bh` 开局库中的候选招法（包含正向与镜像对称查询），并按评分降序排列
    pub fn query(&self, board: &BoardState) -> Vec<BookMove> {
        let red_to_move = board.turn.is_red();
        let normal_hash = ZobristHasher::compute_with_turn(board, red_to_move, false);
        let mirror_hash = ZobristHasher::compute_with_turn(board, red_to_move, true);

        let mut results = Vec::new();
        for book in &self.books {
            results.extend(book.query(normal_hash, false));
            if mirror_hash != normal_hash {
                results.extend(book.query(mirror_hash, true));
            }
        }

        results.sort_by(|a, b| {
            b.score.cmp(&a.score).then(
                b.win_rate
                    .partial_cmp(&a.win_rate)
                    .unwrap_or(Ordering::Equal),
            )
        });

        results
    }

    /// 根据策略执行本地与云库综合异步查询
    pub async fn query_hybrid(&self, board: &BoardState, fen: &str) -> Vec<BookMove> {
        match self.mode {
            CloudBookMode::LocalOnly => self.query(board),
            CloudBookMode::CloudOnly => self.cloud.query(fen).await.unwrap_or_default(),
            CloudBookMode::Hybrid => {
                let local_moves = self.query(board);
                if !local_moves.is_empty() {
                    local_moves
                } else {
                    self.cloud.query(fen).await.unwrap_or_default()
                }
            }
            CloudBookMode::Merge => {
                let mut local_moves = self.query(board);
                let cloud_moves = self.cloud.query(fen).await.unwrap_or_default();

                for cm in cloud_moves {
                    if !local_moves.iter().any(|lm| lm.iccs == cm.iccs) {
                        local_moves.push(cm);
                    }
                }

                local_moves.sort_by(|a, b| {
                    b.score.cmp(&a.score).then(
                        b.win_rate
                            .partial_cmp(&a.win_rate)
                            .unwrap_or(Ordering::Equal),
                    )
                });

                local_moves
            }
        }
    }

    /// 已加载开局库的路径列表
    pub fn loaded_books(&self) -> Vec<String> {
        self.books.iter().map(|b| b.path().to_string()).collect()
    }
}

impl Default for OpeningBookService {
    fn default() -> Self {
        Self::new()
    }
}
