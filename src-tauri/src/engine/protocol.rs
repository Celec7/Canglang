use crate::engine::models::{
    AnalysisConfig, EngineCapabilities, EngineOptionDescriptor, EngineOptionType,
    EngineOptionValue, ThinkData,
};
use std::collections::HashSet;

/// 握手阶段收集的引擎身份、动态选项与原始诊断行
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HandshakeInfo {
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub options: Vec<EngineOptionDescriptor>,
    pub raw_lines: Vec<String>,
}

/// 解析一条 UCI/UCCI 握手行，保留无法结构化的行供诊断使用
pub fn parse_handshake_line(info: &mut HandshakeInfo, line: &str) {
    info.raw_lines.push(line.to_string());

    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() >= 3 && tokens[0].eq_ignore_ascii_case("id") {
        let value = tokens[2..].join(" ");
        if tokens[1].eq_ignore_ascii_case("name") {
            info.name = Some(value);
        } else if tokens[1].eq_ignore_ascii_case("version") {
            info.version = Some(value);
        } else if tokens[1].eq_ignore_ascii_case("author") {
            info.author = Some(value);
        }
        return;
    }

    if let Some(descriptor) = parse_option_descriptor(line) {
        let key = descriptor.name.to_ascii_lowercase();
        if let Some(existing) = info
            .options
            .iter_mut()
            .find(|item| item.name.to_ascii_lowercase() == key)
        {
            *existing = descriptor;
        } else {
            info.options.push(descriptor);
        }
    }
}

/// 将 UCI/UCCI 握手结果转换成保守的能力描述
pub fn capabilities_for_protocol(protocol: &str, info: &HandshakeInfo) -> EngineCapabilities {
    let option_names: HashSet<String> = info
        .options
        .iter()
        .map(|option| option.name.to_ascii_lowercase())
        .collect();
    let raw = info.raw_lines.join(" ").to_ascii_lowercase();

    EngineCapabilities {
        protocol: protocol.to_ascii_lowercase(),
        // UCI 的 searchmoves 是标准 go 扩展；其它协议只有在握手文本明确出现时才认为支持
        searchmoves: protocol.eq_ignore_ascii_case("uci") || raw.contains("searchmoves"),
        banmoves: raw.contains("banmoves"),
        ponder: option_names.contains("ponder"),
        multi_pv: option_names.contains("multipv"),
    }
}

fn parse_option_descriptor(line: &str) -> Option<EngineOptionDescriptor> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 5 || !tokens[0].eq_ignore_ascii_case("option") {
        return None;
    }

    let name_index = tokens
        .iter()
        .position(|token| token.eq_ignore_ascii_case("name"))?;
    let type_index = tokens
        .iter()
        .position(|token| token.eq_ignore_ascii_case("type"))?;
    if name_index + 1 >= type_index || type_index + 1 >= tokens.len() {
        return None;
    }

    let name = tokens[name_index + 1..type_index].join(" ");
    let option_type = match tokens[type_index + 1].to_ascii_lowercase().as_str() {
        "check" => EngineOptionType::Check,
        "spin" => EngineOptionType::Spin,
        "combo" => EngineOptionType::Combo,
        "string" => EngineOptionType::String,
        "file" => EngineOptionType::File,
        "button" => EngineOptionType::Button,
        _ => return None,
    };

    let mut default_text = None;
    let mut min = None;
    let mut max = None;
    let mut vars = Vec::new();
    let mut index = type_index + 2;
    while index < tokens.len() {
        let key = tokens[index].to_ascii_lowercase();
        match key.as_str() {
            "default" => {
                let (value, next) = collect_until_keyword(&tokens, index + 1);
                default_text = Some(value);
                index = next;
            }
            "min" => {
                min = tokens.get(index + 1).and_then(|value| value.parse().ok());
                index += 2;
            }
            "max" => {
                max = tokens.get(index + 1).and_then(|value| value.parse().ok());
                index += 2;
            }
            "var" => {
                let (value, next) = collect_until_keyword(&tokens, index + 1);
                if !value.is_empty() {
                    vars.push(value);
                }
                index = next;
            }
            _ => index += 1,
        }
    }

    let default = default_text.and_then(|value| option_value_from_text(&option_type, &value));
    Some(EngineOptionDescriptor {
        name,
        option_type,
        default,
        min,
        max,
        step: None,
        vars,
    })
}

fn collect_until_keyword(tokens: &[&str], start: usize) -> (String, usize) {
    let mut end = start;
    while end < tokens.len()
        && !matches!(
            tokens[end].to_ascii_lowercase().as_str(),
            "default" | "min" | "max" | "var"
        )
    {
        end += 1;
    }
    (tokens[start..end].join(" "), end)
}

fn option_value_from_text(
    option_type: &EngineOptionType,
    value: &str,
) -> Option<EngineOptionValue> {
    match option_type {
        EngineOptionType::Check => value.parse().ok().map(EngineOptionValue::Bool),
        EngineOptionType::Spin => value.parse().ok().map(EngineOptionValue::Integer),
        EngineOptionType::Combo => Some(EngineOptionValue::Enum(value.to_string())),
        EngineOptionType::String => Some(EngineOptionValue::String(value.to_string())),
        EngineOptionType::File => Some(EngineOptionValue::Path(value.to_string())),
        EngineOptionType::Button => None,
    }
}

/// 引擎通信协议抽象，屏蔽 UCI 与 UCCI 差异
pub trait Protocol: Send + Sync {
    fn name(&self) -> &str;

    fn handshake_command(&self) -> &str;
    fn handshake_ok_marker(&self) -> &str;

    fn ready_command(&self) -> &str;
    fn ready_ok_marker(&self) -> &str;

    /// 格式化局面指令（`position startpos ...` 或 `position fen <fen> ... moves ...`）
    fn format_position(&self, fen: Option<&str>, moves: Option<&[String]>) -> String;

    /// 格式化开始思考指令（支持各种时限/深度模式及排除着法 searchmoves）
    fn format_go(&self, config: &AnalysisConfig, search_moves: Option<&[String]>) -> String;

    /// 格式化仅支持 `banmoves` 方言的根节点约束
    fn format_go_with_banmoves(&self, config: &AnalysisConfig, ban_moves: &[String]) -> String {
        let mut out = self.format_go(config, None);
        let list: Vec<&str> = ban_moves
            .iter()
            .filter(|mv| !mv.trim().is_empty())
            .map(String::as_str)
            .collect();
        if !list.is_empty() {
            out.push_str(" banmoves ");
            out.push_str(&list.join(" "));
        }
        out
    }

    fn format_stop(&self) -> &'static str;
    fn format_quit(&self) -> &'static str;

    fn format_set_option(&self, name: &str, value: &str) -> String;

    /// 格式化无 value 的 button 类型选项
    fn format_button_option(&self, name: &str) -> String;

    /// 从输出行解析 ThinkData（若不是有效 info 行则返回 None）
    fn parse_info(&self, line: &str) -> Option<ThinkData>;

    /// 从输出行解析最佳走法（ICCS 格式，若无或不是 bestmove 行则返回 None）
    fn parse_best_move(&self, line: &str) -> Option<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_identity_and_typed_options() {
        let mut info = HandshakeInfo::default();
        parse_handshake_line(&mut info, "id name Test Engine");
        parse_handshake_line(&mut info, "id author Test Author");
        parse_handshake_line(
            &mut info,
            "option name Hash type spin default 128 min 1 max 4096",
        );
        parse_handshake_line(
            &mut info,
            "option name Style type combo default Normal var Solid var Normal",
        );
        parse_handshake_line(&mut info, "option name Ponder type check default true");

        assert_eq!(info.name.as_deref(), Some("Test Engine"));
        assert_eq!(info.author.as_deref(), Some("Test Author"));
        assert_eq!(info.options.len(), 3);
        assert_eq!(info.options[0].min, Some(1));
        assert_eq!(info.options[0].max, Some(4096));
        assert_eq!(
            info.options[1].vars,
            vec!["Solid".to_string(), "Normal".to_string()]
        );
        assert_eq!(info.options[2].default, Some(EngineOptionValue::Bool(true)));
    }

    #[test]
    fn capabilities_are_conservative_for_unknown_protocol_extensions() {
        let mut info = HandshakeInfo::default();
        parse_handshake_line(
            &mut info,
            "option name MultiPV type spin default 1 min 1 max 8",
        );
        parse_handshake_line(&mut info, "option name Ponder type check default false");

        let capabilities = capabilities_for_protocol("ucci", &info);
        assert!(!capabilities.searchmoves);
        assert!(!capabilities.banmoves);
        assert!(capabilities.multi_pv);
        assert!(capabilities.ponder);

        let uci_capabilities = capabilities_for_protocol("uci", &info);
        assert!(uci_capabilities.searchmoves);
    }
}
