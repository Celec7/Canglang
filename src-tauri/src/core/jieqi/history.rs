use super::{JieqiGameResult, JieqiPosition, PublicPly};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JieqiHistory {
    pub(crate) states: Vec<JieqiPosition>,
    pub(crate) results: Vec<Option<JieqiGameResult>>,
    pub(crate) plies: Vec<PublicPly>,
    pub(crate) cursor: usize,
}

impl JieqiHistory {
    pub(crate) fn new(initial: JieqiPosition) -> Self {
        Self {
            states: vec![initial],
            results: vec![None],
            plies: Vec::new(),
            cursor: 0,
        }
    }

    pub(crate) fn current(&self) -> &JieqiPosition {
        &self.states[self.cursor]
    }

    pub(crate) fn head(&self) -> usize {
        self.plies.len()
    }

    pub(crate) fn truncate_future(&mut self) {
        self.plies.truncate(self.cursor);
        self.states.truncate(self.cursor + 1);
        self.results.truncate(self.cursor + 1);
    }

    pub(crate) fn push(
        &mut self,
        position: JieqiPosition,
        result: Option<JieqiGameResult>,
        ply: PublicPly,
    ) {
        self.plies.push(ply);
        self.states.push(position);
        self.results.push(result);
        self.cursor = self.plies.len();
    }
}
