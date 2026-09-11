import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type {
  ApplyMoveLineRequest, JieqiPlayMode, NewGameOptions, PlyRecord, PreviewRequest,
  PreviewSnapshot, PublicPly, RuleExplanation, RuleProfile, RuleStatus,
  SessionCapabilities, SessionPly, SessionResult, SessionSnapshot, SessionToken,
  JieqiDocumentOpenResult,
} from "@/bindings";
import { commands, SessionCommandError, unwrap, unwrapSession } from "@/lib/ipc";
import { playSound } from "@/lib/sound";
import { useEngineStore } from "./engine";
import { useManualStore } from "./manual";

export interface UnifiedPly {
  ply: number;
  iccs: string;
  notation: string;
  mover: "red" | "black";
  is_capture: boolean;
  is_check: boolean;
  variant: "xiangqi" | "jieqi";
  public_detail?: string;
}

const disabledCapability = { enabled: false, reason: "wrong_variant" as const };
const emptyCapabilities: SessionCapabilities = {
  move: disabledCapability, undo: disabledCapability, redo: disabledCapability,
  jump: disabledCapability, resign: disabledCapability, offer_draw: disabledCapability,
  save_private: disabledCapability, save_public: disabledCapability,
  edit_annotations: disabledCapability, analyze: disabledCapability,
  query_book: disabledCapability, edit_position: disabledCapability, use_fen: disabledCapability,
};

function publicResult(result?: SessionResult): "ongoing" | "redwin" | "blackwin" | "draw" {
  if (!result || result.type === "ongoing") return "ongoing";
  if (result.type === "draw") return "draw";
  return result.winner === "red" ? "redwin" : "blackwin";
}

function normalizePly(entry: SessionPly): UnifiedPly {
  if (entry.variant === "xiangqi") {
    return {
      ...entry.ply,
      mover: entry.ply.mover === "red" ? "red" : "black",
      variant: "xiangqi",
    };
  }
  const kinds: Record<string, string> = {
    king: "将帅", advisor: "士", bishop: "象", knight: "马", rook: "车", cannon: "炮", pawn: "兵卒",
  };
  const details: string[] = [];
  if (entry.ply.revealed) details.push(`揭为${kinds[entry.ply.revealed]}`);
  if (entry.ply.captured.type === "hidden") details.push("吃暗子");
  if (entry.ply.captured.type === "revealed") details.push(`吃${kinds[entry.ply.captured.kind]}`);
  return {
    ply: entry.ply.ply,
    iccs: entry.ply.iccs,
    notation: entry.ply.notation,
    mover: entry.ply.mover,
    is_capture: entry.ply.captured.type !== "none",
    is_check: entry.ply.is_check,
    variant: "jieqi",
    public_detail: details.join("，") || undefined,
  };
}

export const useGameStore = defineStore("game", () => {
  const snapshot = ref<SessionSnapshot | null>(null);
  const lastMove = ref<{ legal: true } | null>(null);
  let mutationQueue: Promise<void> = Promise.resolve();

  const gameId = computed(() => snapshot.value?.game_id ?? null);
  const revision = computed(() => snapshot.value?.revision ?? null);
  const contentRevision = computed(() => snapshot.value?.content_revision ?? null);
  const position = computed(() => snapshot.value?.position ?? null);
  const startPosition = computed(() => snapshot.value?.start_position ?? null);
  const variant = computed(() => snapshot.value?.position.variant ?? null);
  const fen = computed(() => snapshot.value?.position.variant === "xiangqi" ? snapshot.value.position.fen : null);
  const currentFen = fen;
  const startFen = computed(() => snapshot.value?.start_position.variant === "xiangqi" ? snapshot.value.start_position.fen : null);
  const currentPly = computed(() => snapshot.value?.current_ply ?? 0);
  const history = computed(() => (snapshot.value?.history ?? []).map(normalizePly));
  const appliedHistory = computed(() => history.value.slice(0, currentPly.value));
  const futureHistory = computed(() => history.value.slice(currentPly.value));
  const xiangqiHistory = computed<PlyRecord[]>(() =>
    (snapshot.value?.history ?? [])
      .filter((entry): entry is Extract<SessionPly, { variant: "xiangqi" }> => entry.variant === "xiangqi")
      .map((entry) => entry.ply),
  );
  const jieqiHistory = computed<PublicPly[]>(() =>
    (snapshot.value?.history ?? [])
      .filter((entry): entry is Extract<SessionPly, { variant: "jieqi" }> => entry.variant === "jieqi")
      .map((entry) => entry.ply),
  );
  const result = computed(() => publicResult(snapshot.value?.result));
  const redToMove = computed(() => {
    const current = snapshot.value?.position;
    if (!current) return true;
    return current.variant === "jieqi"
      ? current.position.turn === "red"
      : current.fen.trim().split(/\s+/)[1] !== "b";
  });
  const inCheck = computed(() => snapshot.value?.in_check ?? false);
  const capabilities = computed(() => snapshot.value?.capabilities ?? emptyCapabilities);
  const canUndo = computed(() => capabilities.value.undo.enabled);
  const canRedo = computed(() => capabilities.value.redo.enabled);
  const ruleProfile = computed<RuleProfile | null>(() =>
    snapshot.value?.rules.variant === "xiangqi" ? snapshot.value.rules.profile : null,
  );
  const playMode = computed<JieqiPlayMode | null>(() =>
    snapshot.value?.position.variant === "jieqi" ? snapshot.value.play_mode : null,
  );
  const repetitionCount = computed(() => snapshot.value?.xiangqi_assessment?.repetition_count ?? 1);
  const repetitionExplanation = computed(() => snapshot.value?.xiangqi_assessment?.repetition_explanation ?? null);
  const ruleStatus = computed<RuleStatus>(() => snapshot.value?.xiangqi_assessment?.rule_status ?? "ongoing");
  const ruleExplanation = computed<RuleExplanation | null>(() => snapshot.value?.xiangqi_assessment?.rule_explanation ?? null);

  function applySnapshot(next: SessionSnapshot) {
    snapshot.value = next;
  }

  function token(): SessionToken {
    const current = snapshot.value;
    if (!current) throw new Error("对局尚未初始化");
    return { game_id: current.game_id, expected_revision: current.revision };
  }

  async function refresh() {
    applySnapshot(await commands.sessionGet());
  }

  async function init() {
    await refresh();
  }

  function serializeMutation<T>(operation: () => Promise<T>): Promise<T> {
    const pending = mutationQueue.then(operation, operation);
    mutationQueue = pending.then(() => undefined, () => undefined);
    return pending;
  }

  async function mutate(command: (sessionToken: SessionToken) => Promise<SessionSnapshot>) {
    return serializeMutation(async () => {
      try {
        const next = await command(token());
        applySnapshot(next);
        return next;
      } catch (cause) {
        if (cause instanceof SessionCommandError && cause.code === "stale_session") await refresh();
        throw cause;
      }
    });
  }

  async function makeMoveInternal(iccs: string, syncManual: boolean): Promise<{ legal: true }> {
    await mutate(async (sessionToken) => unwrapSession(await commands.sessionMove(sessionToken, iccs)));
    lastMove.value = { legal: true };
    const latestPly = appliedHistory.value[appliedHistory.value.length - 1];
    if (syncManual && variant.value === "xiangqi" && startFen.value) {
      useManualStore().recordHistory(startFen.value, xiangqiHistory.value.slice(0, currentPly.value));
    }
    if (result.value !== "ongoing") playSound("win");
    else if (inCheck.value) playSound("check");
    else if (latestPly?.is_capture) playSound("capture");
    else playSound("move");
    return { legal: true };
  }

  async function makeMove(iccs: string) { return makeMoveInternal(iccs, true); }
  async function replayMove(iccs: string) { return makeMoveInternal(iccs, false); }

  async function targets(row: number, col: number): Promise<string[]> {
    const expected = token();
    try {
      const moves = await unwrapSession(await commands.sessionTargets(expected, { row, col }));
      if (snapshot.value?.game_id !== expected.game_id || snapshot.value.revision !== expected.expected_revision) return [];
      return moves;
    } catch (cause) {
      if (cause instanceof SessionCommandError && cause.code === "stale_session") await refresh();
      throw cause;
    }
  }

  async function previewLine(pv: string[]): Promise<PreviewSnapshot> {
    if (!capabilities.value.analyze.enabled || !startFen.value || !fen.value || !ruleProfile.value) {
      throw new Error("当前对局不支持象棋引擎预览");
    }
    const request: PreviewRequest = {
      start_fen: startFen.value,
      history: appliedHistory.value.map((ply) => ply.iccs),
      pv: [...pv],
      rule_profile: ruleProfile.value,
    };
    return unwrap(await commands.previewLine(request));
  }

  async function applyPreviewPrefix(moves: string[]): Promise<void> {
    if (!fen.value || !startFen.value) throw new Error("当前对局不支持象棋分析变例");
    const request: ApplyMoveLineRequest = { expected_fen: fen.value, moves: [...moves] };
    await unwrap(await commands.applyMoveLine(request));
    await refresh();
    useManualStore().recordHistory(startFen.value, xiangqiHistory.value.slice(0, currentPly.value));
  }

  async function newSession(
    options: NewGameOptions,
    guards: { preserveManual?: boolean; skipManualGuard?: boolean } = {},
  ): Promise<boolean> {
    const manual = useManualStore();
    if (!guards.skipManualGuard && !(await manual.confirmDiscard())) return false;
    await mutate(async (sessionToken) => unwrapSession(await commands.sessionNew(sessionToken, options)));
    if (!guards.preserveManual) manual.clear();
    useEngineStore().clearGameEvaluations();
    return true;
  }

  async function newGame(
    startFenOverride?: string,
    guards: { preserveManual?: boolean; skipManualGuard?: boolean } = {},
  ) {
    return newSession({
      variant: "xiangqi", fen: startFenOverride ?? null,
      rule_profile: ruleProfile.value ?? "china2020",
    }, guards);
  }

  async function setRuleProfile(profile: RuleProfile) {
    if (variant.value !== "xiangqi") return;
    await newSession({ variant: "xiangqi", fen: fen.value, rule_profile: profile }, { preserveManual: true });
  }

  async function undo() { await mutate(async (t) => unwrapSession(await commands.sessionUndo(t))); }
  async function redo() { await mutate(async (t) => unwrapSession(await commands.sessionRedo(t))); }
  async function resign(side: "red" | "black") {
    await mutate(async (t) => unwrapSession(await commands.sessionResign(t, side)));
  }
  async function jumpTo(ply: number) {
    await mutate(async (t) => unwrapSession(await commands.sessionJump(t, ply)));
  }
  async function offerDraw(side: "red" | "black") {
    await mutate(async (t) => unwrapSession(await commands.sessionOfferDraw(t, side)));
  }
  async function respondDraw(offerId: string, side: "red" | "black", accept: boolean) {
    await mutate(async (t) => unwrapSession(await commands.sessionRespondDraw(t, offerId, side, accept)));
  }
  async function cancelDraw(offerId: string, side: "red" | "black") {
    await mutate(async (t) => unwrapSession(await commands.sessionCancelDraw(t, offerId, side)));
  }

  async function openJieqiDocument(path: string): Promise<JieqiDocumentOpenResult> {
    return serializeMutation(async () => {
      try {
        const opened = await unwrapSession(await commands.jieqiDocumentOpen(token(), path));
        applySnapshot(opened.snapshot);
        useEngineStore().clearGameEvaluations();
        return opened;
      } catch (cause) {
        if (cause instanceof SessionCommandError && cause.code === "stale_session") await refresh();
        throw cause;
      }
    });
  }

  return {
    snapshot, gameId, revision, contentRevision, position, startPosition, variant,
    fen, startFen, currentFen, currentPly, result, redToMove, inCheck, ruleProfile,
    playMode, repetitionCount, repetitionExplanation, ruleStatus, ruleExplanation,
    history, xiangqiHistory, jieqiHistory, appliedHistory, futureHistory,
    capabilities, canUndo, canRedo, lastMove,
    init, refresh, token, targets, makeMove, replayMove, previewLine, applyPreviewPrefix,
    newSession, newGame, setRuleProfile, undo, redo, resign, jumpTo,
    offerDraw, respondDraw, cancelDraw, openJieqiDocument,
  };
});
