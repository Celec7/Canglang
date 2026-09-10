import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type {
  ApplyMoveLineRequest,
  GameSnapshot,
  MoveResult,
  PlyRecord,
  PreviewRequest,
  PreviewSnapshot,
  RuleExplanation,
  RuleProfile,
  RuleStatus,
} from "@/bindings";
import { commands, unwrap } from "@/lib/ipc";
import { playSound } from "@/lib/sound";
import { useManualStore } from "./manual";
import { useEngineStore } from "./engine";

export const useGameStore = defineStore("game", () => {
  const fen = ref("");
  const startFen = ref("");
  const currentFen = ref("");
  const currentPly = ref(0);
  const result = ref("ongoing");
  const redToMove = ref(true);
  const inCheck = ref(false);
  const ruleProfile = ref<RuleProfile>("china2020");
  const repetitionCount = ref(1);
  const repetitionExplanation = ref<string | null>(null);
  const ruleStatus = ref<RuleStatus>("ongoing");
  const ruleExplanation = ref<RuleExplanation | null>(null);
  const history = ref<PlyRecord[]>([]);
  const canUndo = ref(false);
  const canRedo = ref(false);
  const lastMove = ref<MoveResult | null>(null);

  const appliedHistory = computed(() => history.value.slice(0, currentPly.value));
  const futureHistory = computed(() => history.value.slice(currentPly.value));

  function applySnapshot(s: GameSnapshot) {
    fen.value = s.current_fen || s.fen;
    currentFen.value = s.current_fen || s.fen;
    startFen.value = s.start_fen || s.current_fen;
    currentPly.value = s.current_ply;
    result.value = s.result;
    redToMove.value = s.red_to_move;
    inCheck.value = s.in_check;
    ruleProfile.value = s.rule_profile;
    repetitionCount.value = s.repetition_count;
    repetitionExplanation.value = s.repetition_explanation;
    ruleStatus.value = s.rule_status;
    ruleExplanation.value = s.rule_explanation;
    history.value = s.history;
    canUndo.value = s.can_undo;
    canRedo.value = s.can_redo;
  }

  async function refresh() {
    applySnapshot(await commands.gameResult());
  }

  async function init() {
    fen.value = await commands.getInitialBoard();
    currentFen.value = fen.value;
    startFen.value = fen.value;
    await refresh();
  }

  async function makeMoveInternal(iccs: string, syncManual: boolean): Promise<MoveResult> {
    const res = await unwrap(await commands.makeMove(fen.value, iccs));
    lastMove.value = res;
    if (res.legal) {
      // `refresh()` 一次性返回包含新 FEN、历史和游标的 GameSnapshot
      // 在快照到达前不要发布新 FEN，否则生命周期监听器可能把新 FEN 与旧历史一起发送给引擎
      // 后端会正确拒绝这个不一致的上下文
      await refresh();

      const latestPly = history.value[currentPly.value - 1];
      if (syncManual) {
        useManualStore().recordHistory(startFen.value, appliedHistory.value);
      }
      if (res.game_over) {
        playSound("win");
      } else if (res.check || inCheck.value) {
        playSound("check");
      } else if (latestPly?.is_capture) {
        playSound("capture");
      } else {
        playSound("move");
      }
    }
    return res;
  }

  async function makeMove(iccs: string): Promise<MoveResult> {
    return makeMoveInternal(iccs, true);
  }

  // 重放已有变例，但不修改当前加载的棋谱
  async function replayMove(iccs: string): Promise<MoveResult> {
    return makeMoveInternal(iccs, false);
  }

  async function previewLine(pv: string[]): Promise<PreviewSnapshot> {
    const request: PreviewRequest = {
      start_fen: startFen.value,
      history: appliedHistory.value.map((ply) => ply.iccs),
      pv: [...pv],
      rule_profile: ruleProfile.value,
    };
    return unwrap(await commands.previewLine(request));
  }

  async function applyPreviewPrefix(moves: string[]): Promise<void> {
    const request: ApplyMoveLineRequest = {
      expected_fen: currentFen.value,
      moves: [...moves],
    };
    applySnapshot(await unwrap(await commands.applyMoveLine(request)));
    useManualStore().recordHistory(startFen.value, appliedHistory.value);
  }

  async function newGame(
    startFenOverride?: string,
    options: { preserveManual?: boolean; skipManualGuard?: boolean } = {},
  ): Promise<boolean> {
    const manual = useManualStore();
    if (!options.skipManualGuard && !manual.confirmDiscard()) return false;
    applySnapshot(await unwrap(await commands.newGame(startFenOverride ?? null)));
    if (!options.preserveManual) manual.clear();
    useEngineStore().clearGameEvaluations();
    return true;
  }

  async function setRuleProfile(profile: RuleProfile) {
    applySnapshot(await commands.setRuleProfile(profile));
  }

  async function undo() {
    applySnapshot(await commands.undoMove());
  }

  async function redo() {
    applySnapshot(await commands.redoMove());
  }

  async function resign(side: "red" | "black") {
    applySnapshot(await unwrap(await commands.resign(side)));
  }

  /// 原子跳转到指定步数游标（0 = 初始局面）
  async function jumpTo(ply: number) {
    applySnapshot(await commands.jumpTo(ply));
  }

  return {
    fen,
    startFen,
    currentFen,
    currentPly,
    result,
    redToMove,
    inCheck,
    ruleProfile,
    repetitionCount,
    repetitionExplanation,
    ruleStatus,
    ruleExplanation,
    history,
    appliedHistory,
    futureHistory,
    canUndo,
    canRedo,
    lastMove,
    init,
    refresh,
    makeMove,
    replayMove,
    previewLine,
    applyPreviewPrefix,
    newGame,
    setRuleProfile,
    undo,
    redo,
    resign,
    jumpTo,
  };
});
