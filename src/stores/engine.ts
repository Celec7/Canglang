import { defineStore } from "pinia";
import { computed, onScopeDispose, ref } from "vue";
import type {
  AnalysisConfig,
  AnalysisRequest,
  AnalysisStartResult,
  EngineConfig,
  EngineInfo,
  PreviewSnapshot,
  RootMoveConstraint,
} from "@/bindings";
import type { Turn } from "@/lib/chess";
import {
  autoMoveConfig,
  isPositionCurrent,
  shouldPlayBestMove,
  type AnalysisContext,
} from "@/lib/engine-analysis";
import { scoreToWinRate, toRedPerspective } from "@/lib/engine-evaluation";
import { listenBestMove, listenEngineStopped, listenThink, commands, unwrap, type UnlistenFn } from "@/lib/ipc";
import type { ThinkData } from "@/lib/events";
import { useGameStore } from "@/stores/game";

// ThinkData 仅通过 think:// 事件传递，Rust 模型是权威来源；此处保留兼容导出
export type { ThinkData } from "@/lib/events";

export interface PlyEvaluation {
  ply: number;
  score: number;
  mateIn: number | null;
  winRate: number;
  /** 用于标识局面的 ICCS 历史，避免分支后沿用过期值 */
  positionKey: string;
  iccs?: string;
  chinese?: string;
}

export const useEngineStore = defineStore("engine", () => {
  const defaultAnalysisConfig: AnalysisConfig = {
    mode: "fixed_time",
    value: 1000,
    multi_pv: 1,
    engine_delay_ms: 0,
    book_delay_ms: 0,
  };
  const running = ref(false);
  const analyzing = ref(false);
  const analysisEnabled = ref(true);
  const engineInfo = ref<EngineInfo | null>(null);
  const lastError = ref<string | null>(null);

  // Multi-PV 映射：键为 multi_pv 序号（1、2、3……）
  const multiPvMap = ref<Record<number, ThinkData>>({});
  const multiPvList = computed(() =>
    Object.values(multiPvMap.value)
      .filter(
        (td) =>
          td.multi_pv <= analysisConfig.value.multi_pv &&
          td.pv &&
          td.pv.length > 0
      )
      .sort((a, b) => a.multi_pv - b.multi_pv)
  );
  const latestThink = computed<ThinkData | null>(() => multiPvMap.value[1] ?? multiPvList.value[0] ?? null);

  // 整局各步（Ply）评估记录，键为步数（0 ~ N）
  const gameEvaluations = ref<Record<number, PlyEvaluation>>({});
  // 当前搜索尚未返回新结果时，界面继续显示最近一次已完成的评估
  const lastEvaluation = ref<PlyEvaluation | null>(null);

  const evaluations = ref<ThinkData[]>([]);
  const bestMove = ref<string | null>(null);
  const lastAnalysisSessionId = ref<string | null>(null);
  const analysisConstraint = ref<RootMoveConstraint>({ candidates: [], banned: [], temporary_excluded: [] });
  const constraintStatus = ref<AnalysisStartResult["constraint_status"]>("not_applied");
  const bestMoveAction = ref<"move" | "variation" | null>(null);
  const previewSnapshot = ref<PreviewSnapshot | null>(null);
  const previewMoves = ref<string[]>([]);
  const analysisConfig = ref<AnalysisConfig>({ ...defaultAnalysisConfig });
  const autoMoveRed = ref(false);
  const autoMoveBlack = ref(false);

  const isAnalysisOnly = computed(() => !autoMoveRed.value && !autoMoveBlack.value);

  function isControlledTurn(turn: Turn): boolean {
    return (turn === "red" && autoMoveRed.value) || (turn === "black" && autoMoveBlack.value);
  }

  let thinkUnlisten: UnlistenFn | null = null;
  let bestUnlisten: UnlistenFn | null = null;
  let stoppedUnlisten: UnlistenFn | null = null;
  let autoMoveTimer: ReturnType<typeof setTimeout> | null = null;
  let activeAnalysis: AnalysisContext | null = null;

  function errorMessage(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
  }

  async function subscribe() {
    if (!thinkUnlisten) {
      thinkUnlisten = await listenThink((event) => {
        if (event.kind !== "think") return;
        const td = event.payload.data;
        if (!td.pv || td.pv.length === 0) return;

        if (
          !activeAnalysis ||
          event.payload.analysis_session_id !== activeAnalysis.analysisSessionId
        ) {
          return;
        }

        const game = useGameStore();
        if (game.fen !== activeAnalysis.fen) {
          return;
        }

        const historyLen = game.appliedHistory.length;
        if (historyLen !== activeAnalysis.history.length) {
          return;
        }

        multiPvMap.value = { ...multiPvMap.value, [td.multi_pv]: td };
        if (td.multi_pv === 1) {
          evaluations.value = [...evaluations.value.slice(-119), td];
          const currentPly = historyLen;
          const lastMove = game.appliedHistory[historyLen - 1];
          const redPerspective = toRedPerspective(td.score, td.mate_in, game.redToMove);
          const positionKey = activeAnalysis.history.join(" ");
          gameEvaluations.value = {
            ...gameEvaluations.value,
            [currentPly]: {
              ply: currentPly,
              score: redPerspective.score,
              mateIn: redPerspective.mateIn,
              winRate: scoreToWinRate(redPerspective.score),
              positionKey,
              iccs: lastMove?.iccs,
              chinese: lastMove?.notation,
            },
          };
          lastEvaluation.value = gameEvaluations.value[currentPly];
        }
      });
    }
    if (!bestUnlisten) {
      bestUnlisten = await listenBestMove(async (event) => {
        if (event.kind !== "best_move") return;
        const context = activeAnalysis;
        if (
          !context ||
          event.payload.analysis_session_id !== context.analysisSessionId
        ) {
          return;
        }

        const game = useGameStore();
        if (game.fen !== context.fen || game.appliedHistory.length !== context.history.length) {
          return;
        }

        activeAnalysis = null;
        bestMove.value = event.payload.move_iccs;
        analyzing.value = false;

        const currentTurn = game.redToMove ? "red" : "black";
        const action = context.action;
        bestMoveAction.value = null;

        const isControlled = isControlledTurn(currentTurn);
        const shouldPlay = shouldPlayBestMove(action, isControlled);
        const isAtLiveTip = game.appliedHistory.length === game.history.length;
        if (shouldPlay && game.result === "ongoing" && isAtLiveTip) {
          let repetitionPending = false;
          try {
            repetitionPending = (await game.previewLine([event.payload.move_iccs])).rule_status === "repetition_pending";
          } catch {
            // 非法或不可用的预览不能让规则辅助器变成裁判器
          }
          const hasManualConstraint = context.constraint.candidates.length > 0 || context.constraint.banned.length > 0;
          if (repetitionPending && context.retryCount === 0 && !hasManualConstraint) {
            lastError.value = "引擎着法可能造成重复，正在排除该着法重搜";
            try {
              await changeTactic(
                game.fen,
                context.history,
                [...context.constraint.temporary_excluded, event.payload.move_iccs],
                analysisConfig.value,
                "move",
                1,
              );
            } catch {
              // changeTactic 已经在 store 中记录了可处理的错误
            }
            return;
          }
          if (repetitionPending && context.retryCount > 0) {
            bestMove.value = event.payload.move_iccs;
            bestMoveAction.value = null;
            lastError.value = "引擎连续返回可能造成重复的着法，请手动接受或换一变";
            return;
          }
          bestMove.value = null;
          multiPvMap.value = {};
          try {
            await game.makeMove(event.payload.move_iccs);
          } catch (cause) {
            // 自动引擎着法没有发起操作的组件可接收异常，将失败保留在工作区可观察的 store 状态中
            lastError.value = errorMessage(cause);
          }
        }
      });
    }
    if (!stoppedUnlisten) {
      stoppedUnlisten = await listenEngineStopped(() => {
        cancelAutoMove();
        activeAnalysis = null;
        running.value = false;
        analyzing.value = false;
        multiPvMap.value = {};
        evaluations.value = [];
        bestMove.value = null;
        bestMoveAction.value = null;
        engineInfo.value = null;
        autoMoveRed.value = false;
        autoMoveBlack.value = false;
        lastError.value = "引擎进程已退出";
      });
    }
  }

  async function start(config: EngineConfig) {
    lastError.value = null;
    await subscribe();
    engineInfo.value = await unwrap(await commands.engineStart(config));
    running.value = true;
    activeAnalysis = null;
    analyzing.value = false;
    multiPvMap.value = {};
    evaluations.value = [];
    bestMove.value = null;
    bestMoveAction.value = null;
    lastAnalysisSessionId.value = null;
    analysisConstraint.value = { candidates: [], banned: [], temporary_excluded: [] };
    constraintStatus.value = "not_applied";
    clearPreview();
    const game = useGameStore();
    if (analysisEnabled.value && game.result === "ongoing") {
      const history = game.appliedHistory.map((ply) => ply.iccs);
      if (isControlledTurn(game.redToMove ? "red" : "black")) {
        scheduleAutoMove(game.fen, history, game.redToMove ? "red" : "black", 0);
      } else {
        void analyze(game.fen, history, analysisConfig.value).catch(() => undefined);
      }
    }
  }

  function newAnalysisSessionId(): string {
    return globalThis.crypto?.randomUUID?.() ?? `analysis-${Date.now()}-${Math.random()}`;
  }

  function makeAnalysisRequest(
    analysisSessionId: string,
    fen: string,
    history: string[],
    config: AnalysisConfig,
    constraint: RootMoveConstraint = { candidates: [], banned: [], temporary_excluded: [] },
  ): AnalysisRequest {
    return {
      analysis_session_id: analysisSessionId,
      position: {
        start_fen: useGameStore().startFen,
        history: [...history],
        current_fen: fen,
      },
      constraint,
      config,
    };
  }

  async function analyze(
    fen: string,
    history: string[],
    config: AnalysisConfig,
    action: "move" | null = null
  ) {
    if (!analysisEnabled.value && action !== "move") {
      return;
    }
    analysisConfig.value = { ...config };
    lastError.value = null;
    bestMove.value = null;
    bestMoveAction.value = action;
    multiPvMap.value = {};
    analyzing.value = true;
    const analysisSessionId = newAnalysisSessionId();
    lastAnalysisSessionId.value = analysisSessionId;
    const game = useGameStore();
    activeAnalysis = {
      analysisSessionId,
      startFen: game.startFen,
      fen,
      history: [...history],
      ruleProfile: game.ruleProfile,
      constraint: { candidates: [], banned: [], temporary_excluded: [] },
      retryCount: 0,
      action,
    };
    analysisConstraint.value = { candidates: [], banned: [], temporary_excluded: [] };
    constraintStatus.value = "not_applied";
    try {
      const response = await unwrap(
        await commands.engineAnalyze(makeAnalysisRequest(analysisSessionId, fen, history, config)),
      );
      constraintStatus.value = response.constraint_status;
      if (!response.started) {
        activeAnalysis = null;
        analyzing.value = false;
        bestMoveAction.value = null;
        lastError.value = "当前引擎不支持所请求的根节点约束";
      }
    } catch (cause) {
      activeAnalysis = null;
      analyzing.value = false;
      bestMoveAction.value = null;
      lastError.value = errorMessage(cause);
      throw cause;
    }
  }

  async function moveNow(options: { reportError?: boolean } = {}) {
    bestMoveAction.value = "move";
    lastError.value = null;
    try {
      await unwrap(await commands.engineMoveNow());
    } catch (cause) {
      bestMoveAction.value = null;
      if (options.reportError) {
        lastError.value = errorMessage(cause);
      }
      throw cause;
    }
  }

  async function changeTactic(
    fen: string,
    history: string[],
    excluded: string[],
    config: AnalysisConfig,
    action: "variation" | "move" = "variation",
    retryCount = 0,
  ) {
    if (analyzing.value) return;
    analysisConfig.value = { ...config };
    lastError.value = null;
    bestMove.value = null;
    bestMoveAction.value = action;
    multiPvMap.value = {};
    analyzing.value = true;
    const analysisSessionId = newAnalysisSessionId();
    lastAnalysisSessionId.value = analysisSessionId;
    const game = useGameStore();
    const constraint = { candidates: [], banned: [], temporary_excluded: [...excluded] };
    activeAnalysis = {
      analysisSessionId,
      startFen: game.startFen,
      fen,
      history: [...history],
      ruleProfile: game.ruleProfile,
      constraint,
      action,
      retryCount,
    };
    analysisConstraint.value = constraint;
    constraintStatus.value = "not_applied";
    try {
      const response = await unwrap(
        await commands.engineChangeTactic(
          makeAnalysisRequest(analysisSessionId, fen, history, config, {
            candidates: [],
            banned: [],
            temporary_excluded: [...excluded],
          }),
        ),
      );
      constraintStatus.value = response.constraint_status;
      if (!response.started) {
        activeAnalysis = null;
        analyzing.value = false;
        bestMoveAction.value = null;
        lastError.value = "当前引擎不支持所请求的根节点约束";
      }
    } catch (cause) {
      activeAnalysis = null;
      analyzing.value = false;
      bestMoveAction.value = null;
      lastError.value = errorMessage(cause);
      throw cause;
    }
  }

  async function analyzeWithConstraint(constraint: RootMoveConstraint) {
    if (!running.value || analyzing.value) return;
    const game = useGameStore();
    const history = game.appliedHistory.map((ply) => ply.iccs);
    const analysisSessionId = newAnalysisSessionId();
    lastAnalysisSessionId.value = analysisSessionId;
    analysisConstraint.value = {
      candidates: [...constraint.candidates],
      banned: [...constraint.banned],
      temporary_excluded: [...constraint.temporary_excluded],
    };
    constraintStatus.value = "not_applied";
    lastError.value = null;
    bestMove.value = null;
    bestMoveAction.value = null;
    multiPvMap.value = {};
    analyzing.value = true;
    activeAnalysis = {
      analysisSessionId,
      startFen: game.startFen,
      fen: game.fen,
      history: [...history],
      ruleProfile: game.ruleProfile,
      constraint: analysisConstraint.value,
      retryCount: 0,
      action: null,
    };
    try {
      const response = await unwrap(await commands.engineAnalyze(
        makeAnalysisRequest(analysisSessionId, game.fen, history, analysisConfig.value, analysisConstraint.value),
      ));
      constraintStatus.value = response.constraint_status;
      if (!response.started) {
        activeAnalysis = null;
        analyzing.value = false;
        lastError.value = "当前引擎不支持所请求的根节点约束";
      }
    } catch (cause) {
      activeAnalysis = null;
      analyzing.value = false;
      lastError.value = errorMessage(cause);
      throw cause;
    }
  }

  async function previewPv(pv: string[]) {
    const game = useGameStore();
    previewMoves.value = [...pv];
    previewSnapshot.value = await game.previewLine(pv);
  }

  function clearPreview() {
    previewMoves.value = [];
    previewSnapshot.value = null;
  }

  async function applyPreview() {
    if (previewMoves.value.length === 0) return;
    await useGameStore().applyPreviewPrefix(previewMoves.value);
    clearPreview();
  }

  function setAnalysisConfig(config: AnalysisConfig) {
    analysisConfig.value = { ...config };
  }

  function setAnalysisEnabled(enabled: boolean) {
    analysisEnabled.value = enabled;
    if (!enabled) {
      cancelAutoMove();
      if (analyzing.value) void stopAnalysis();
      return;
    }
    if (enabled) reconcileControlMode();
  }

  async function stopAnalysis() {
    activeAnalysis = null;
    analyzing.value = false;
    cancelAutoMove();
    multiPvMap.value = {};
    bestMove.value = null;
    bestMoveAction.value = null;
    lastError.value = null;
    if (running.value) {
      try {
        await unwrap(await commands.engineMoveNow());
      } catch {
        // 停止已经结束的分析属于预期的清理竞争
      }
    }
  }

  function consumeBestMove(): { iccs: string; action: "move" | "variation" } | null {
    if (!bestMove.value || !bestMoveAction.value) return null;
    const result = { iccs: bestMove.value, action: bestMoveAction.value };
    bestMove.value = null;
    bestMoveAction.value = null;
    return result;
  }

  function cancelAutoMove() {
    if (autoMoveTimer !== null) {
      clearTimeout(autoMoveTimer);
      autoMoveTimer = null;
    }
  }

  function scheduleAutoMove(fen: string, history: string[], turn: Turn, animationDelayMs: number = 0) {
    cancelAutoMove();
    if (!running.value || !analysisEnabled.value || !isControlledTurn(turn)) return;

    const game = useGameStore();
    if (game.result !== "ongoing" || game.appliedHistory.length < game.history.length) {
      return;
    }

    autoMoveTimer = setTimeout(() => {
      autoMoveTimer = null;
      if (!running.value || !analysisEnabled.value || !isControlledTurn(turn)) return;
      if (game.result !== "ongoing" || game.appliedHistory.length < game.history.length) {
        return;
      }
      if (!isPositionCurrent({ fen, history }, game.fen, game.appliedHistory.map((ply) => ply.iccs))) {
        return;
      }

      // 如果当前分析限制为无限思考，自动走子不能使用无限，降级为默认限时 1500ms
      const config = autoMoveConfig(analysisConfig.value);

      void analyze(fen, history, config, "move").catch(() => {
        // analyze() 会将失败写入 lastError；此处只清理待执行的自动走子，因为没有调用方可重新抛出异常
        bestMoveAction.value = null;
      });
    }, Math.max(0, animationDelayMs));
  }

  function reconcileControlMode() {
    cancelAutoMove();
    if (!running.value || !analysisEnabled.value) return;

    const game = useGameStore();
    if (game.result !== "ongoing" || game.appliedHistory.length < game.history.length) return;

    const currentTurn = game.redToMove ? "red" : "black";
    const iccsHistory = game.appliedHistory.map((p) => p.iccs);

    if (isControlledTurn(currentTurn)) {
      if (analyzing.value) {
        // 复用正在进行的搜索，但将结果标记为自动走子
        bestMoveAction.value = "move";
        if (analysisConfig.value.mode === "infinite") {
          void moveNow({ reportError: true }).catch(() => {
            // moveNow() 会为工作区将失败写入 lastError
          });
        }
      } else {
        scheduleAutoMove(game.fen, iccsHistory, currentTurn, 0);
      }
      return;
    }

    // 如果待执行的自动搜索刚切换回仅分析模式，立即替换其上下文，避免迟到的 bestmove 执行走子
    if (analyzing.value && bestMoveAction.value === "move") {
      void analyze(game.fen, iccsHistory, analysisConfig.value).catch(() => undefined);
    }
  }

  function triggerControlledTurnIfNeeded() {
    reconcileControlMode();
  }

  function toggleAutoMoveRed() {
    analysisEnabled.value = true;
    autoMoveRed.value = !autoMoveRed.value;
    triggerControlledTurnIfNeeded();
  }

  function toggleAutoMoveBlack() {
    analysisEnabled.value = true;
    autoMoveBlack.value = !autoMoveBlack.value;
    triggerControlledTurnIfNeeded();
  }

  function setAnalysisOnly() {
    analysisEnabled.value = true;
    autoMoveRed.value = false;
    autoMoveBlack.value = false;
    reconcileControlMode();
  }

  function clearGameEvaluations() {
    cancelAutoMove();
    activeAnalysis = null;
    analysisConstraint.value = { candidates: [], banned: [], temporary_excluded: [] };
    constraintStatus.value = "not_applied";
    gameEvaluations.value = {};
    lastEvaluation.value = null;
    analyzing.value = false;
    multiPvMap.value = {};
    evaluations.value = [];
    bestMove.value = null;
    bestMoveAction.value = null;
    lastError.value = null;
    clearPreview();
  }

  async function stop() {
    cancelAutoMove();
    activeAnalysis = null;
    analysisConstraint.value = { candidates: [], banned: [], temporary_excluded: [] };
    constraintStatus.value = "not_applied";
    try {
      await unwrap(await commands.engineStop());
    } finally {
      running.value = false;
      analyzing.value = false;
      multiPvMap.value = {};
      evaluations.value = [];
      bestMove.value = null;
      bestMoveAction.value = null;
      lastError.value = null;
      engineInfo.value = null;
      autoMoveRed.value = false;
      autoMoveBlack.value = false;
      clearPreview();
    }
  }

  onScopeDispose(() => {
    cancelAutoMove();
    activeAnalysis = null;
    const unlisteners = [thinkUnlisten, bestUnlisten, stoppedUnlisten];
    thinkUnlisten = null;
    bestUnlisten = null;
    stoppedUnlisten = null;
    void Promise.all(
      unlisteners
        .filter((fn): fn is UnlistenFn => fn !== null)
        .map((fn) => fn())
    );
  });

  return {
    running,
    analyzing,
    analysisEnabled,
    engineInfo,
    latestThink,
    multiPvMap,
    multiPvList,
    previewSnapshot,
    previewMoves,
    gameEvaluations,
    lastEvaluation,
    evaluations,
    bestMove,
    lastAnalysisSessionId,
    analysisConstraint,
    constraintStatus,
    bestMoveAction,
    lastError,
    analysisConfig,
    autoMoveRed,
    autoMoveBlack,
    isAnalysisOnly,
    isControlledTurn,
    toggleAutoMoveRed,
    toggleAutoMoveBlack,
    setAnalysisOnly,
    setAnalysisEnabled,
    stopAnalysis,
    start,
    analyze,
    moveNow,
    changeTactic,
    analyzeWithConstraint,
    previewPv,
    clearPreview,
    applyPreview,
    setAnalysisConfig,
    consumeBestMove,
    scheduleAutoMove,
    cancelAutoMove,
    clearGameEvaluations,
    stop,
  };
});
