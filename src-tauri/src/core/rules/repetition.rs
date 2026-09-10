use super::RuleProfile;

/// 当前局面是否进入需要规则裁判介入的重复循环
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepetitionAssessment {
    pub count: u32,
    pub cycle_length: Option<u32>,
    pub explanation: Option<String>,
}

pub fn assess_repetition(
    profile: RuleProfile,
    count: u32,
    cycle_length: Option<u32>,
) -> RepetitionAssessment {
    let explanation = if count >= 3 {
        Some(match profile {
            RuleProfile::China2020 => {
                "同一局面已出现三次：按中国规则 2020 需结合将、捉、兑、献、拦、闲等行为裁判。"
                    .to_string()
            }
            RuleProfile::Asian2017 => {
                "同一局面已出现三次：按亚洲规则 2017 需结合循环行为裁判，不能直接套用自动和棋。"
                    .to_string()
            }
        })
    } else {
        None
    };

    RepetitionAssessment {
        count,
        cycle_length,
        explanation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repetition_requires_judgement_after_three_occurrences() {
        assert!(
            assess_repetition(RuleProfile::China2020, 2, None)
                .explanation
                .is_none()
        );
        assert!(
            assess_repetition(RuleProfile::China2020, 3, Some(4))
                .explanation
                .is_some()
        );
        assert!(
            assess_repetition(RuleProfile::Asian2017, 3, Some(4))
                .explanation
                .unwrap()
                .contains("亚洲规则")
        );
    }
}
