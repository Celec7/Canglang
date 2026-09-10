use canglang_app::core::board::INITIAL_FEN;
use canglang_app::manual::XqfParser;

fn append_v10_record(data: &mut Vec<u8>, from: u8, to: u8, flags: u8, comment: &str) {
    let encoded = encoding_rs::GBK.encode(comment).0;
    data.extend([from, to, flags, 0]);
    data.extend((encoded.len() as i32).to_le_bytes());
    data.extend_from_slice(&encoded);
}

fn xqf_v10_with_comments() -> Vec<u8> {
    let mut buf = vec![0u8; 1024];
    buf[0] = b'X';
    buf[1] = b'Q';
    buf[2] = 10;

    let standard_pieces: [u8; 32] = [
        0, 10, 20, 30, 40, 50, 60, 70, 80, 12, 72, 3, 23, 43, 63, 83, 9, 19, 29, 39, 49, 59, 69,
        79, 89, 17, 77, 6, 26, 46, 66, 86,
    ];
    buf[16..48].copy_from_slice(&standard_pieces);

    let mut data = Vec::new();
    append_v10_record(&mut data, 0, 0, 0x80, "起始局面说明");
    append_v10_record(&mut data, 72 + 24, 42 + 32, 0, "第一手说明");
    buf.extend(data);
    buf
}

fn xqf_v10_with_illegal_move() -> Vec<u8> {
    let mut buf = xqf_v10_with_comments();
    // 保持起始格为 h2，但将目标格也改为 h2
    // 字节内容为 v10 未加密的 from/to 值及其字段偏移
    buf[1024 + 20 + 1] = 72 + 32;
    buf
}

#[test]
fn parse_invalid_magic_throws_error() {
    let invalid = [0x00, 0x01, 0x02];
    assert!(XqfParser::parse(&invalid).is_err());
}

#[test]
fn parse_too_short_throws_error() {
    let mut short_buf = vec![0u8; 100];
    short_buf[0] = b'X';
    short_buf[1] = b'Q';
    assert!(XqfParser::parse(&short_buf).is_err());
}

#[test]
fn parse_unencrypted_xqf_with_trailing_null_in_string_truncates_cleanly() {
    let mut buf = vec![0u8; 1024];
    buf[0] = b'X';
    buf[1] = b'Q';
    buf[2] = 10; // version <= 10 (no encryption)

    // 32 个棋子位于标准初始局面的坐标，顺序与 FEN_PIECES 一致
    let standard_pieces: [u8; 32] = [
        0, 10, 20, 30, 40, 50, 60, 70, 80, 12, 72, 3, 23, 43, 63, 83, // 红
        9, 19, 29, 39, 49, 59, 69, 79, 89, 17, 77, 6, 26, 46, 66, 86, // 黑
    ];
    buf[16..48].copy_from_slice(&standard_pieces);

    // 设置带尾随 \0 和脏字节的标题:
    // 偏移 80：首个字节是声明长度（10）
    // 后面是 "测试棋谱" (GBK 8 字节) + '\0' + 脏字符 0xFF
    buf[80] = 10;
    let title_bytes = encoding_rs::GBK.encode("测试棋谱").0;
    buf[81..81 + title_bytes.len()].copy_from_slice(&title_bytes);
    buf[81 + title_bytes.len()] = 0; // '\0'
    buf[81 + title_bytes.len() + 1] = 0xFF; // 脏字节

    let manual = XqfParser::parse(&buf).unwrap();

    assert_eq!(manual.title, "测试棋谱");
    assert!(!manual.title.contains('\0'));
    assert_eq!(manual.start_fen, INITIAL_FEN);
}

#[test]
fn parse_unencrypted_xqf_preserves_root_and_move_comments() {
    let manual = XqfParser::parse(&xqf_v10_with_comments()).unwrap();

    assert_eq!(manual.root.comment.as_deref(), Some("起始局面说明"));
    assert_eq!(manual.root.children.len(), 1);
    let move_node = &manual.root.children[0];
    assert_eq!(move_node.mv.unwrap().to_iccs(), "h2e2");
    assert_eq!(move_node.comment.as_deref(), Some("第一手说明"));
}

#[test]
fn parse_unencrypted_xqf_rejects_illegal_move() {
    let error = XqfParser::parse(&xqf_v10_with_illegal_move()).unwrap_err();

    assert!(error.to_string().contains("illegal move 'h2h2'"));
}
