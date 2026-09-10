use crate::engine::models::{AnalysisConfig, ThinkData};
use crate::engine::protocol::Protocol;
use crate::engine::protocols::common::{
    format_go_command, format_position_command, parse_best_move_line, parse_info_line,
};

/// UCCI (Universal Chinese Chess Interface) 协议实现
pub struct UcciProtocol;

impl UcciProtocol {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UcciProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Protocol for UcciProtocol {
    fn name(&self) -> &str {
        "ucci"
    }

    fn handshake_command(&self) -> &str {
        "ucci"
    }

    fn handshake_ok_marker(&self) -> &str {
        "ucciok"
    }

    fn ready_command(&self) -> &str {
        "isready"
    }

    fn ready_ok_marker(&self) -> &str {
        "readyok"
    }

    fn format_position(&self, fen: Option<&str>, moves: Option<&[String]>) -> String {
        format_position_command(fen, moves)
    }

    fn format_go(&self, config: &AnalysisConfig, search_moves: Option<&[String]>) -> String {
        format_go_command(config, search_moves, "time")
    }

    fn format_stop(&self) -> &'static str {
        "stop"
    }

    fn format_quit(&self) -> &'static str {
        "quit"
    }

    fn format_set_option(&self, name: &str, value: &str) -> String {
        format!("setoption {name} {value}")
    }

    fn format_button_option(&self, name: &str) -> String {
        format!("setoption {name}")
    }

    fn parse_info(&self, line: &str) -> Option<ThinkData> {
        parse_info_line(line, true)
    }

    fn parse_best_move(&self, line: &str) -> Option<String> {
        parse_best_move_line(line)
    }
}
