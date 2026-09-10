use crate::core::hash::ZobristHasher;
use crate::engine::error::EngineError;
use crate::engine::models::BookMove;
use rusqlite::{Connection, OpenFlags, params};
use std::path::Path;

/// 开局库访问接口
pub trait IOpeningBook: Send {
    /// 开局库名称（通常为文件名）
    fn name(&self) -> &str;

    /// 开局库文件绝对路径
    fn path(&self) -> &str;

    /// 根据指定局面哈希查询开局库招法
    fn query(&self, hash: u64, mirrored: bool) -> Vec<BookMove>;
}

/// 基于 SQLite 的 `.bh` 格式开局库读取器（只读）
/// 严格采用参数化 SQL，正/负哈希分别按 integer/double 匹配，杜绝注入与小数点本地化问题
pub struct BhOpenBook {
    conn: Connection,
    name: String,
    path: String,
}

impl BhOpenBook {
    pub fn open(book_path: &str) -> Result<Self, EngineError> {
        if book_path.trim().is_empty() {
            return Err(EngineError::InvalidConfig(
                "book path cannot be empty".to_string(),
            ));
        }

        let name = Path::new(book_path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| book_path.to_string());

        let conn = Connection::open_with_flags(book_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|source| EngineError::Sqlite { source })?;

        Ok(Self {
            conn,
            name,
            path: book_path.to_string(),
        })
    }

    /// 测试专用：以已打开的连接与名称构造
    #[allow(dead_code)]
    pub fn from_connection(conn: Connection, name: impl Into<String>) -> Self {
        let path = conn.path().map(|p| p.to_string()).unwrap_or_default();
        Self {
            conn,
            name: name.into(),
            path,
        }
    }
}

impl IOpeningBook for BhOpenBook {
    fn name(&self) -> &str {
        &self.name
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn query(&self, hash: u64, mirrored: bool) -> Vec<BookMove> {
        let signed = hash as i64;
        let sql = if signed < 0 {
            "SELECT vscore, vwin, vdraw, vlost, vmemo, vmove FROM bhobk WHERE cast(vkey as double) = ?1 AND vvalid = 1"
        } else {
            "SELECT vscore, vwin, vdraw, vlost, vmemo, vmove FROM bhobk WHERE cast(vkey as integer) = ?1 AND vvalid = 1"
        };

        let mut stmt = match self.conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let mut rows = if signed < 0 {
            match stmt.query(params![f64::from_bits(hash)]) {
                Ok(r) => r,
                Err(_) => return Vec::new(),
            }
        } else {
            match stmt.query(params![signed]) {
                Ok(r) => r,
                Err(_) => return Vec::new(),
            }
        };

        let mut results = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let score = row.get::<_, Option<i32>>(0).ok().flatten().unwrap_or(0);
            let win = row.get::<_, Option<i32>>(1).ok().flatten().unwrap_or(0);
            let draw = row.get::<_, Option<i32>>(2).ok().flatten().unwrap_or(0);
            let lose = row.get::<_, Option<i32>>(3).ok().flatten().unwrap_or(0);
            let note = row.get::<_, Option<String>>(4).ok().flatten();
            let vmove = row.get::<_, Option<i32>>(5).ok().flatten().unwrap_or(0);

            let total = i64::from(win)
                .saturating_add(i64::from(draw))
                .saturating_add(i64::from(lose));
            let win_rate = if total > 0 {
                // 与原始 Java 一致：截断而非四舍五入
                (10000.0 * (win as f64 + draw as f64 / 2.0) / total as f64) as i64 as f64 / 100.0
            } else {
                0.0
            };

            if let Some(mv) = ZobristHasher::get_move_from_vmove(vmove as u16, mirrored) {
                results.push(BookMove {
                    iccs: mv.to_iccs(),
                    notation: None,
                    score,
                    win_count: win.max(0) as u32,
                    draw_count: draw.max(0) as u32,
                    lose_count: lose.max(0) as u32,
                    win_rate,
                    note,
                    source: self.name.clone(),
                });
            }
        }

        results
    }
}
