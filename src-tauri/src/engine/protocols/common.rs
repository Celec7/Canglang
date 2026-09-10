use crate::engine::models::{AnalysisConfig, AnalysisMode, ThinkData};

/// 标准搜索信息关键词列表（用于终止 PV 着法收集）
pub const INFO_KEYWORDS: &[&str] = &[
    "depth",
    "seldepth",
    "multipv",
    "score",
    "nodes",
    "nps",
    "time",
    "pv",
    "hashfull",
    "tbhits",
    "currmove",
    "currmovenumber",
    "string",
    "refutation",
];

/// 格式化 position 命令（UCI / UCCI 通用）
pub fn format_position_command(fen: Option<&str>, moves: Option<&[String]>) -> String {
    let mut out = String::new();
    match fen {
        None => out.push_str("position startpos"),
        Some(f) if f.trim().eq_ignore_ascii_case("startpos") => out.push_str("position startpos"),
        Some(f) => {
            out.push_str("position fen ");
            out.push_str(f.trim());
        }
    }

    if let Some(moves) = moves {
        let list: Vec<&str> = moves
            .iter()
            .filter(|m| !m.trim().is_empty())
            .map(|m| m.as_str())
            .collect();
        if !list.is_empty() {
            out.push_str(" moves ");
            out.push_str(&list.join(" "));
        }
    }

    out
}

/// 格式化 go 命令（UCI 使用 "movetime"，UCCI 使用 "time"）
pub fn format_go_command(
    config: &AnalysisConfig,
    search_moves: Option<&[String]>,
    time_keyword: &str,
) -> String {
    let mut out = String::from("go ");

    match config.mode {
        AnalysisMode::FixedTime => {
            out.push_str(time_keyword);
            out.push(' ');
            out.push_str(&config.value.max(1).to_string());
        }
        AnalysisMode::FixedDepth => {
            out.push_str("depth ");
            out.push_str(&config.value.max(1).to_string());
        }
        AnalysisMode::FixedNodes => {
            out.push_str("nodes ");
            out.push_str(&config.value.max(1).to_string());
        }
        AnalysisMode::Infinite => out.push_str("infinite"),
    }

    if let Some(search_moves) = search_moves {
        let list: Vec<&str> = search_moves
            .iter()
            .filter(|m| !m.trim().is_empty())
            .map(|m| m.as_str())
            .collect();
        if !list.is_empty() {
            out.push_str(" searchmoves ");
            out.push_str(&list.join(" "));
        }
    }

    out
}

/// 解析 bestmove 输出行（UCI / UCCI 通用）
pub fn parse_best_move_line(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if line.eq_ignore_ascii_case("nobestmove") {
        return None;
    }

    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() >= 2 && tokens[0].eq_ignore_ascii_case("bestmove") {
        let mv = tokens[1];
        if mv.eq_ignore_ascii_case("nobestmove")
            || mv.eq_ignore_ascii_case("(none)")
            || mv.eq_ignore_ascii_case("null")
        {
            return None;
        }
        return Some(mv.to_string());
    }

    None
}

/// 解析 info 思考输出行
///
/// `allow_direct_score`: UCCI 支持形如 `score 35` 的直接数字格式，UCI 标准仅支持 `score cp 35` / `score mate 3`
pub fn parse_info_line(line: &str, allow_direct_score: bool) -> Option<ThinkData> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 2 || !tokens[0].eq_ignore_ascii_case("info") {
        return None;
    }

    // `info string ...` 为引擎公告行，不属于搜索更新的 ThinkData
    if tokens
        .get(1)
        .is_some_and(|t| t.eq_ignore_ascii_case("string"))
    {
        return None;
    }

    let mut depth: u32 = 0;
    let mut score: i32 = 0;
    let mut mate_in: Option<i32> = None;
    let mut multi_pv: u32 = 1;
    let mut nps: u64 = 0;
    let mut time_ms: u64 = 0;
    let mut pv: Vec<String> = Vec::new();

    let mut i = 1;
    while i < tokens.len() {
        let token = tokens[i].to_ascii_lowercase();

        if token == "depth" && i + 1 < tokens.len() {
            if let Ok(d) = tokens[i + 1].parse::<u32>() {
                depth = d;
                i += 1;
            }
        } else if token == "multipv" && i + 1 < tokens.len() {
            if let Ok(m) = tokens[i + 1].parse::<u32>() {
                multi_pv = m;
                i += 1;
            }
        } else if token == "score" && i + 1 < tokens.len() {
            let next_token = tokens[i + 1].to_ascii_lowercase();
            if next_token == "cp" && i + 2 < tokens.len() {
                if let Ok(cp) = tokens[i + 2].parse::<i32>() {
                    score = cp;
                    i += 2;
                }
            } else if next_token == "mate" && i + 2 < tokens.len() {
                if let Ok(mate) = tokens[i + 2].parse::<i32>() {
                    mate_in = Some(mate);
                    score = if mate > 0 {
                        30000 - mate
                    } else {
                        -30000 - mate
                    };
                    i += 2;
                }
            } else if allow_direct_score && let Ok(direct) = tokens[i + 1].parse::<i32>() {
                score = direct;
                i += 1;
            }
        } else if token == "nps" && i + 1 < tokens.len() {
            if let Ok(n) = tokens[i + 1].parse::<u64>() {
                nps = n;
                i += 1;
            }
        } else if token == "time" && i + 1 < tokens.len() {
            if let Ok(t) = tokens[i + 1].parse::<u64>() {
                time_ms = t;
                i += 1;
            }
        } else if token == "pv" {
            let mut j = i + 1;
            while j < tokens.len() {
                let t = tokens[j].to_ascii_lowercase();
                if INFO_KEYWORDS.contains(&t.as_str()) {
                    break;
                }
                pv.push(tokens[j].to_string());
                j += 1;
            }
            i = j - 1;
        }

        i += 1;
    }

    if pv.is_empty() {
        return None;
    }

    Some(ThinkData::new(
        depth, score, mate_in, multi_pv, nps, time_ms, pv,
    ))
}
