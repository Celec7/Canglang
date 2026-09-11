use crate::core::game::SessionSnapshot;
use crate::core::jieqi::{JieqiPlayMode, JieqiPublicKind};
use crate::core::piece::{Color, PieceKind};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum JieqiDocumentKind {
    PrivateGame,
    PublicReplay,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct JieqiDocumentMetadata {
    pub title: String,
    pub date: String,
    pub red_player: String,
    pub black_player: String,
    pub event_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JieqiFileIdentity {
    pub piece_id: u8,
    pub kind: PieceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JieqiDocumentInitial {
    Private { assignments: Vec<JieqiFileIdentity> },
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JieqiDocumentMove {
    pub iccs: String,
    pub revealed: Option<JieqiPublicKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JieqiDocumentTermination {
    Resignation { side: Color, ply: u32 },
    DrawAgreement { ply: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JieqiDocument {
    pub format: String,
    pub version: u32,
    pub kind: JieqiDocumentKind,
    pub rules: String,
    pub play_mode: JieqiPlayMode,
    pub metadata: JieqiDocumentMetadata,
    pub initial: JieqiDocumentInitial,
    pub moves: Vec<JieqiDocumentMove>,
    #[serde(deserialize_with = "deserialize_annotations")]
    pub annotations: BTreeMap<u32, String>,
    pub termination: Option<JieqiDocumentTermination>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct JieqiDocumentPublic {
    pub kind: JieqiDocumentKind,
    pub metadata: JieqiDocumentMetadata,
    pub annotations: BTreeMap<u32, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct JieqiDocumentReceipt {
    pub game_id: String,
    pub content_revision: String,
    pub edit_revision: String,
    pub kind: JieqiDocumentKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct JieqiDocumentOpenResult {
    pub snapshot: SessionSnapshot,
    pub document: JieqiDocumentPublic,
}

fn deserialize_annotations<'de, D>(deserializer: D) -> Result<BTreeMap<u32, String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct AnnotationVisitor;
    impl<'de> serde::de::Visitor<'de> for AnnotationVisitor {
        type Value = BTreeMap<u32, String>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("以半回合数为键的备注对象")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                let ply = key.parse::<u32>().map_err(serde::de::Error::custom)?;
                if result.insert(ply, value).is_some() {
                    return Err(serde::de::Error::custom(format!("重复备注键: {key}")));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(AnnotationVisitor)
}
