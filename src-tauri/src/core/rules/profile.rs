use serde::{Deserialize, Serialize};

/// 对局使用的象棋竞赛规则档案
/// `china_2020` 对应中国象棋协会 2020 竞赛规则；`asian_2017` 对应亚洲象棋
/// 联合会 2017 规则，并作为世界规则兼容档案的基础。具体循环裁判由规则服务
/// 根据该档案解释，棋盘走法仍由 `MoveValidator` 统一负责
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum RuleProfile {
    #[default]
    China2020,
    Asian2017,
}

impl RuleProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::China2020 => "中国规则 2020",
            Self::Asian2017 => "亚洲规则 2017",
        }
    }
}
