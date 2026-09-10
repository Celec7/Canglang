use serde::{Deserialize, Serialize};
use specta_typescript::Number;
use std::collections::HashMap;

/// 引擎动态选项的声明类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EngineOptionType {
    Check,
    Spin,
    Combo,
    String,
    File,
    Button,
}

/// 引擎动态选项的持久化值
/// 按引擎握手声明的类型保存，只有在协议边界才转换成字符串
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum EngineOptionValue {
    Bool(bool),
    Integer(i32),
    Float(f64),
    Enum(String),
    String(String),
    Path(String),
}

/// 引擎握手声明的动态选项
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct EngineOptionDescriptor {
    pub name: String,
    pub option_type: EngineOptionType,
    pub default: Option<EngineOptionValue>,
    pub min: Option<i32>,
    pub max: Option<i32>,
    pub step: Option<i32>,
    pub vars: Vec<String>,
}

/// 引擎协议与根节点约束能力
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct EngineCapabilities {
    pub protocol: String,
    pub searchmoves: bool,
    pub banmoves: bool,
    pub ponder: bool,
    pub multi_pv: bool,
}

/// 根节点约束应用结果，供前端显示真实状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintApplicationStatus {
    Applied,
    NotApplied,
    Unsupported,
}

/// 分析启动结果，明确告知前端根节点约束是否真的生效
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct AnalysisStartResult {
    pub constraint_status: ConstraintApplicationStatus,
    pub started: bool,
}

/// 引擎协议使用的局面上下文
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct EnginePositionContext {
    pub start_fen: String,
    pub history: Vec<String>,
    pub current_fen: String,
}

/// 当前分析请求的根节点着法约束
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct RootMoveConstraint {
    pub candidates: Vec<String>,
    pub banned: Vec<String>,
    pub temporary_excluded: Vec<String>,
}

/// 引擎思考限制模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AnalysisMode {
    #[default]
    FixedTime,
    FixedDepth,
    FixedNodes,
    Infinite,
}

/// 外部象棋引擎配置
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct EngineConfig {
    pub path: String,
    pub protocol: String,
    pub options: Option<HashMap<String, String>>,
    pub nnue_path: Option<String>,
}

/// 引擎握手后确认的身份与协议诊断信息
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct EngineInfo {
    pub name: Option<String>,
    pub protocol: String,
    pub ready: bool,
    pub diagnostic: Option<String>,
    pub option_descriptors: Vec<EngineOptionDescriptor>,
    pub capabilities: EngineCapabilities,
}

impl EngineConfig {
    pub fn new(path: impl Into<String>, protocol: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            protocol: protocol.into(),
            options: None,
            nnue_path: None,
        }
    }

    pub fn with_options(mut self, options: HashMap<String, String>) -> Self {
        self.options = Some(options);
        self
    }

    pub fn with_nnue(mut self, nnue_path: impl Into<String>) -> Self {
        self.nnue_path = Some(nnue_path.into());
        self
    }
}

/// 引擎思考与变例分析配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct AnalysisConfig {
    pub mode: AnalysisMode,
    #[specta(type = Number)]
    pub value: u64,
    pub multi_pv: u32,
    #[specta(type = Number)]
    pub engine_delay_ms: u64,
    #[specta(type = Number)]
    pub book_delay_ms: u64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self::new(AnalysisMode::FixedTime, 1000)
    }
}

impl AnalysisConfig {
    pub fn new(mode: AnalysisMode, value: u64) -> Self {
        Self {
            mode,
            value,
            multi_pv: 1,
            engine_delay_ms: 0,
            book_delay_ms: 0,
        }
    }

    pub fn with_multi_pv(mut self, multi_pv: u32) -> Self {
        self.multi_pv = multi_pv;
        self
    }
}

/// 引擎实时思考输出数据（对应 info 命令行）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ThinkData {
    pub depth: u32,
    pub score: i32,
    pub mate_in: Option<i32>,
    pub multi_pv: u32,
    #[specta(type = Number)]
    pub nps: u64,
    #[specta(type = Number)]
    pub time_ms: u64,
    pub pv: Vec<String>,
    /// 由 Session 层结合当前局面推导的中文走法序列
    pub pv_chinese: Vec<String>,
}

impl ThinkData {
    pub fn new(
        depth: u32,
        score: i32,
        mate_in: Option<i32>,
        multi_pv: u32,
        nps: u64,
        time_ms: u64,
        pv: Vec<String>,
    ) -> Self {
        Self {
            depth,
            score,
            mate_in,
            multi_pv,
            nps,
            time_ms,
            pv,
            pv_chinese: Vec::new(),
        }
    }
}

/// 前端发起的一次外部引擎分析请求
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct AnalysisRequest {
    pub analysis_session_id: String,
    pub position: EnginePositionContext,
    pub constraint: RootMoveConstraint,
    pub config: AnalysisConfig,
}

/// 带分析会话身份的实时引擎事件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum AnalysisEvent {
    Think {
        analysis_session_id: String,
        data: ThinkData,
    },
    BestMove {
        analysis_session_id: String,
        move_iccs: String,
    },
}

/// 开局库招法条目
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct BookMove {
    pub iccs: String,
    pub notation: Option<String>,
    pub score: i32,
    pub win_count: u32,
    pub draw_count: u32,
    pub lose_count: u32,
    pub win_rate: f64,
    pub note: Option<String>,
    pub source: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_option_value_round_trips_with_stable_tags() {
        let value = EngineOptionValue::Integer(16);
        let json = serde_json::to_string(&value).expect("动态选项值应可序列化");
        assert_eq!(json, r#"{"kind":"integer","value":16}"#);

        let decoded: EngineOptionValue =
            serde_json::from_str(&json).expect("动态选项值应可反序列化");
        assert_eq!(decoded, value);
    }

    #[test]
    fn analysis_request_contains_session_and_position_contract() {
        let request = AnalysisRequest {
            analysis_session_id: "session-1".to_string(),
            position: EnginePositionContext {
                start_fen: "startpos".to_string(),
                history: vec!["h2e2".to_string()],
                current_fen: "current".to_string(),
            },
            constraint: RootMoveConstraint::default(),
            config: AnalysisConfig::default(),
        };

        assert_eq!(request.analysis_session_id, "session-1");
        assert_eq!(request.position.history, vec!["h2e2"]);
    }
}
