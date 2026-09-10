use encoding_rs::GBK;

/// 将字节序列解码为字符串（BOM 检查 → 严格 UTF-8 → GBK 回落）
pub fn decode_pgn_text(bytes: &[u8]) -> String {
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }

    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }

    decode_gbk(bytes)
}

/// 以 GBK (CP936) 解码字节序列为 UTF-8 字符串
pub fn decode_gbk(bytes: &[u8]) -> String {
    GBK.decode(bytes).0.into_owned()
}

/// 严格将 UTF-8 文本编码为 GBK；无法表示的字符返回调用方错误信息
pub fn encode_gbk_strict(
    text: &str,
    field: &str,
) -> Result<Vec<u8>, crate::manual::error::ManualError> {
    let (encoded, _, had_errors) = GBK.encode(text);
    if had_errors {
        return Err(crate::manual::error::ManualError::InvalidData {
            msg: format!("XQF {field} contains characters that cannot be represented in GBK"),
        });
    }
    Ok(encoded.into_owned())
}

/// 从缓冲区指定偏移读取「首字节为长度 + GBK 内容」的字符串，并在首个 `\0` 处截断
pub fn read_gbk_string(buffer: &[u8], offset: usize, max_length: usize) -> String {
    if max_length == 0 || offset >= buffer.len() {
        return String::new();
    }

    let length = buffer[offset] as usize;
    if length == 0 || length > max_length - 1 {
        return String::new();
    }

    let mut actual = 0usize;
    for i in 0..length {
        let idx = offset + 1 + i;
        if idx >= buffer.len() {
            break;
        }
        if buffer[idx] != 0 {
            actual += 1;
        } else {
            break; // 遇 `\0` 截断
        }
    }

    if actual == 0 {
        return String::new();
    }

    decode_gbk(&buffer[offset + 1..offset + 1 + actual])
        .trim()
        .to_string()
}
