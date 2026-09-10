use canglang_app::core::board::BoardState;
use canglang_app::engine::IOpeningBook;
use canglang_app::engine::book::{BhOpenBook, OpeningBookService};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const RED_HASH: u64 = 0x628D04D7C9C144AE;
const BLACK_HASH: u64 = 0xC2432E2EC5846BF6;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 生成一个唯一的临时 `.bh` 文件路径，并在 Drop 时清理
struct TempDb {
    path: PathBuf,
}

impl TempDb {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("canglang_{tag}_{}_{n}.bh", std::process::id()));
        Self { path }
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

type BookEntry<'a> = (u64, bool, i32, i32, i32, i32, &'a str, i32);

fn create_book_db(path: &Path, entries: &[BookEntry<'_>]) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE bhobk (vkey, vvalid INTEGER, vscore INTEGER, vwin INTEGER, vdraw INTEGER, vlost INTEGER, vmemo TEXT, vmove INTEGER);",
    )
    .unwrap();

    for (hash, store_as_integer, score, win, draw, lose, memo, vmove) in entries {
        let key: rusqlite::types::Value = if *store_as_integer {
            rusqlite::types::Value::Integer(*hash as i64)
        } else {
            rusqlite::types::Value::Real(f64::from_bits(*hash))
        };
        conn.execute(
            "INSERT INTO bhobk (vkey, vvalid, vscore, vwin, vdraw, vlost, vmemo, vmove) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![key, score, win, draw, lose, memo, vmove],
        )
        .unwrap();
    }
}

#[test]
fn bh_open_book_queries_positive_hash_returns_move() {
    let db = TempDb::new("pos");
    create_book_db(
        &db.path,
        &[(
            RED_HASH,
            true,
            100,
            50,
            30,
            20,
            "中炮开局",
            (0xaa << 8) | 0xa7,
        )],
    );

    let book = BhOpenBook::open(db.path.to_str().unwrap()).unwrap();
    let moves = book.query(RED_HASH, false);

    assert_eq!(moves.len(), 1);
    let m = &moves[0];
    assert_eq!(m.iccs, "h2e2");
    assert_eq!(m.score, 100);
    assert_eq!(m.win_count, 50);
    assert_eq!(m.draw_count, 30);
    assert_eq!(m.lose_count, 20);
    assert_eq!(m.win_rate, 65.0);
    assert_eq!(m.note.as_deref(), Some("中炮开局"));
}

#[test]
fn bh_open_book_queries_negative_hash_returns_move() {
    let db = TempDb::new("neg");
    create_book_db(
        &db.path,
        &[(
            BLACK_HASH,
            false,
            90,
            40,
            40,
            20,
            "屏风马应中炮",
            (0x3a << 8) | 0x59,
        )],
    );

    let book = BhOpenBook::open(db.path.to_str().unwrap()).unwrap();
    let moves = book.query(BLACK_HASH, false);

    assert_eq!(moves.len(), 1);
    let m = &moves[0];
    assert_eq!(m.iccs, "h9g7");
    assert_eq!(m.score, 90);
    assert_eq!(m.win_rate, 60.0);
    assert_eq!(m.note.as_deref(), Some("屏风马应中炮"));
}

#[test]
fn opening_book_service_query_integrates_with_board() {
    let db = TempDb::new("svc");
    create_book_db(
        &db.path,
        &[(
            RED_HASH,
            true,
            100,
            50,
            30,
            20,
            "中炮开局",
            (0xaa << 8) | 0xa7,
        )],
    );

    let mut service = OpeningBookService::new();
    service.load_book(db.path.to_str().unwrap()).unwrap();

    let board = BoardState::initial();
    let moves = service.query(&board);

    assert!(!moves.is_empty());
    assert_eq!(moves[0].iccs, "h2e2");
}

#[test]
fn opening_book_service_multiple_books_sorts_by_score_descending() {
    let db1 = TempDb::new("sort1");
    create_book_db(
        &db1.path,
        &[(
            RED_HASH,
            true,
            100,
            50,
            30,
            20,
            "中炮开局",
            (0xaa << 8) | 0xa7,
        )],
    );

    let db2 = TempDb::new("sort2");
    create_book_db(
        &db2.path,
        &[(
            RED_HASH,
            true,
            150,
            60,
            20,
            20,
            "高分中炮",
            (0xaa << 8) | 0xa7,
        )],
    );

    let mut service = OpeningBookService::new();
    service.load_book(db1.path.to_str().unwrap()).unwrap();
    service.load_book(db2.path.to_str().unwrap()).unwrap();

    let board = BoardState::initial();
    let moves = service.query(&board);

    assert_eq!(moves.len(), 2);
    assert_eq!(moves[0].score, 150);
    assert_eq!(moves[1].score, 100);
}

#[test]
fn opening_book_rejects_empty_path_and_empty_service_is_safe() {
    let mut service = OpeningBookService::new();

    assert!(service.load_book(" ").is_err());
    assert!(service.query(&BoardState::initial()).is_empty());
}

#[test]
fn malformed_book_schema_and_move_encoding_produce_no_candidates() {
    let schema_db = TempDb::new("bad_schema");
    let conn = Connection::open(&schema_db.path).unwrap();
    conn.execute("CREATE TABLE not_bhobk (value INTEGER)", [])
        .unwrap();
    drop(conn);

    let book = BhOpenBook::open(schema_db.path.to_str().unwrap()).unwrap();
    assert!(book.query(RED_HASH, false).is_empty());

    let invalid_move_db = TempDb::new("bad_move");
    create_book_db(
        &invalid_move_db.path,
        &[(RED_HASH, true, 10, 1, 0, 0, "invalid move", 0)],
    );
    let book = BhOpenBook::open(invalid_move_db.path.to_str().unwrap()).unwrap();
    assert!(book.query(RED_HASH, false).is_empty());
}

#[test]
fn extreme_book_counts_do_not_overflow_query_statistics() {
    let db = TempDb::new("extreme_counts");
    create_book_db(
        &db.path,
        &[(
            RED_HASH,
            true,
            10,
            i32::MAX,
            i32::MAX,
            i32::MAX,
            "large counts",
            (0xaa << 8) | 0xa7,
        )],
    );

    let book = BhOpenBook::open(db.path.to_str().unwrap()).unwrap();
    let moves = book.query(RED_HASH, false);
    assert_eq!(moves.len(), 1);
    assert!(moves[0].win_rate.is_finite());
}
