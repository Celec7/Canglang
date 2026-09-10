import { onMounted, watch } from "vue";
import { resultLabel } from "@/lib/presentation";
import { engineConfigFromProfile } from "@/lib/engine-profile";
import { useA11yAnnouncer } from "@/composables/useA11yAnnouncer";
import { useBookStore } from "@/stores/book";
import { useEngineStore } from "@/stores/engine";
import { useGameStore } from "@/stores/game";
import { usePreferencesStore } from "@/stores/preferences";

function applyTheme(theme: "light" | "dark") {
  if (typeof document === "undefined") return;
  document.documentElement.classList.toggle("dark", theme === "dark");
}

export function useAppLifecycle() {
  const game = useGameStore();
  const preferences = usePreferencesStore();
  const engine = useEngineStore();
  const book = useBookStore();
  const { politeMessage, assertiveMessage, announce } = useA11yAnnouncer();

  onMounted(async () => {
    await preferences.init().catch(() => undefined);
    applyTheme(preferences.theme);

    engine.setAnalysisConfig({
      mode: preferences.analysisMode,
      value: preferences.analysisLimitValue,
      multi_pv: preferences.multiPv,
      engine_delay_ms: 0,
      book_delay_ms: 0,
    });

    try {
      await game.init();
      if (game.ruleProfile !== preferences.defaultRuleProfile) {
        await game.setRuleProfile(preferences.defaultRuleProfile);
      }
    } catch (cause) {
      engine.lastError = `初始化对局失败：${cause instanceof Error ? cause.message : String(cause)}`;
      return;
    }

    const activeProfile =
      preferences.engineProfiles.find((profile) => profile.id === preferences.activeEngineId) ??
      preferences.engineProfiles[0];
    const engineConfig = activeProfile ? engineConfigFromProfile(activeProfile) : null;
    if (engineConfig) {
      void engine.start(engineConfig).catch(() => undefined);
    }

    await book.setCloudEnabled(preferences.cloudBookEnabled).catch(() => undefined);
    await book.setCloudMode(preferences.cloudBookMode).catch(() => undefined);

    if (preferences.openingBookPaths.length > 0) {
      for (const path of preferences.openingBookPaths) {
        await book.load(path).catch(() => undefined);
      }
    }
  });

  watch(
    () => [preferences.cloudBookEnabled, preferences.cloudBookMode] as const,
    async ([enabled, mode]) => {
      await book.setCloudEnabled(enabled).catch(() => undefined);
      await book.setCloudMode(mode).catch(() => undefined);
      if (game.capabilities.query_book.enabled && game.fen) {
        void book.query(game.fen);
      } else {
        book.clearQuery();
      }
    }
  );

  watch(
    () => preferences.theme,
    (theme) => applyTheme(theme)
  );

  watch(
    () => [
      game.fen,
      game.redToMove,
      game.currentPly,
      engine.running,
      engine.analysisEnabled,
      game.result,
    ] as const,
    ([fen, redToMove, , running, analysisEnabled]) => {
      if (
        !running || !analysisEnabled || !fen || game.result !== "ongoing" ||
        !game.capabilities.analyze.enabled
      ) {
        engine.cancelAutoMove();
        engine.clearPositionResults();
        if (engine.analyzing) {
          void engine.stopAnalysis();
        }
        return;
      }

      const isAtLiveTip = game.appliedHistory.length === game.history.length;
      const iccsHistory = game.appliedHistory.map((ply) => ply.iccs);
      const turn = redToMove ? "red" : "black";

      if (!isAtLiveTip) {
        engine.cancelAutoMove();
        void engine.analyze(fen, iccsHistory, engine.analysisConfig).catch(() => undefined);
        return;
      }

      const animationDelayMs = preferences.animations
        ? Math.round(preferences.moveAnimationSeconds * 1000)
        : 0;

      if (engine.isControlledTurn(turn)) {
        engine.scheduleAutoMove(fen, iccsHistory, turn, animationDelayMs);
      } else {
        engine.cancelAutoMove();
        void engine.analyze(fen, iccsHistory, engine.analysisConfig).catch(() => undefined);
      }
    }
  );

  watch(
    () => game.appliedHistory.length,
    (length, previousLength) => {
      if (length > (previousLength ?? 0)) {
        const lastPly = game.appliedHistory[length - 1];
        if (lastPly) {
          const side = lastPly.mover === "red" ? "红方" : "黑方";
          const capture = lastPly.is_capture ? "，吃子" : "";
          announce(`${side} ${lastPly.notation}${capture}`);
        }
        if (game.inCheck && game.result === "ongoing") {
          announce("将军！", "assertive");
        }
      } else if (length < (previousLength ?? 0)) {
        announce(`悔棋，当前第 ${game.currentPly} 步`);
      }
    }
  );

  watch(
    () => game.result,
    (result) => {
      if (result !== "ongoing") {
        announce(`对局结束，${resultLabel(result)}`, "assertive");
      }
    }
  );

  watch(
    () => preferences.boardOrientation,
    (orientation) => {
      const label =
        orientation === "red" ? "红方视角" : orientation === "black" ? "黑方视角" : "跟随行棋视角";
      announce(`棋盘已切换为 ${label}`);
    }
  );

  return {
    politeMessage,
    assertiveMessage,
  };
}
