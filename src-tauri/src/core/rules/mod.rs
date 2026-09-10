pub mod adjudicator;
mod attacks;
pub mod effects;
mod generation;
mod legality;
pub mod profile;
pub mod repetition;

/// 中国象棋规则验证器的稳定公共门面
///
/// 具体实现按攻击检测、候选走法生成和完整合法性判定拆分在内部模块中
pub struct MoveValidator;

pub use adjudicator::{RuleAssessment, RuleExplanation, RuleExplanationCode, RuleStatus};
pub use effects::MoveEffect;
pub use profile::RuleProfile;
