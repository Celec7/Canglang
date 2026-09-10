import { onBeforeUnmount, onMounted } from "vue";
import { isInputActive } from "@/lib/shortcuts";
import { useGameStore } from "@/stores/game";
import { usePreferencesStore } from "@/stores/preferences";
import { useEngineStore } from "@/stores/engine";

export interface ShortcutHandlers {
  onNewGame?: () => void;
  onOpenManual?: () => void;
  onOpenFen?: () => void;
  onOpenSettings?: () => void;
  onToggleShortcutsHelp?: () => void;
  onToggleAnalysisPanel?: () => void;
  onToggleMoveListPanel?: () => void;
  onTogglePlayback?: () => void;
  hasBoardSelection?: () => boolean;
}

export function useShortcuts(handlers: ShortcutHandlers = {}) {
  const game = useGameStore();
  const preferences = usePreferencesStore();
  const engine = useEngineStore();

  function flipBoard() {
    if (preferences.boardOrientation === "red") {
      preferences.setBoardOrientation("black");
    } else if (preferences.boardOrientation === "black") {
      preferences.setBoardOrientation("red");
    } else {
      preferences.setBoardOrientation("red");
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (isInputActive(event.target)) {
      return;
    }

    const mod = event.ctrlKey || event.metaKey;
    const shift = event.shiftKey;
    const key = event.key;

    // 1. 通用与帮助
    if (key === "F1" || (key === "?" && !mod)) {
      event.preventDefault();
      handlers.onToggleShortcutsHelp?.();
      return;
    }

    if (mod && key === ",") {
      event.preventDefault();
      handlers.onOpenSettings?.();
      return;
    }

    // 2. 对局管理 (Ctrl+N, Ctrl+O, Ctrl+Shift+F)
    if (mod && !shift && (key === "n" || key === "N")) {
      event.preventDefault();
      handlers.onNewGame?.();
      return;
    }

    if (mod && !shift && (key === "o" || key === "O")) {
      event.preventDefault();
      handlers.onOpenManual?.();
      return;
    }

    if (mod && shift && (key === "f" || key === "F")) {
      event.preventDefault();
      handlers.onOpenFen?.();
      return;
    }

    // 3. 复盘与撤销重做 ([ / ], Ctrl+Left/Right, Ctrl+Z, Ctrl+Y, Home, End)
    if ((mod && !shift && (key === "z" || key === "Z")) || key === "[" || (mod && key === "ArrowLeft")) {
      event.preventDefault();
      if (game.canUndo) void game.undo();
      return;
    }

    if (
      (mod && shift && (key === "z" || key === "Z")) ||
      (mod && !shift && (key === "y" || key === "Y")) ||
      key === "]" ||
      (mod && key === "ArrowRight")
    ) {
      event.preventDefault();
      if (game.canRedo) void game.redo();
      return;
    }

    if (key === "Home" || (shift && key === "{") || (shift && key === "[")) {
      event.preventDefault();
      if (game.capabilities.jump.enabled) void game.jumpTo(0);
      return;
    }

    if (key === "End" || (shift && key === "}") || (shift && key === "]")) {
      event.preventDefault();
      if (game.capabilities.jump.enabled) void game.jumpTo(game.history.length);
      return;
    }

    // 4. 空格键复盘自动播放 / 暂停 (当棋盘无棋子选中且非对话框时)
    if (key === " " && !mod && !shift) {
      const isBoardSelected = handlers.hasBoardSelection?.() ?? false;
      if (!isBoardSelected) {
        event.preventDefault();
        handlers.onTogglePlayback?.();
        return;
      }
    }

    // 5. 棋盘视角与坐标单键控制 (F, C)
    if (!mod && !shift && (key === "f" || key === "F")) {
      event.preventDefault();
      flipBoard();
      return;
    }

    if (!mod && !shift && (key === "c" || key === "C")) {
      event.preventDefault();
      preferences.showCoordinates = !preferences.showCoordinates;
      return;
    }

    // 6. 引擎分析与面板折叠 (E / Ctrl+E, Ctrl+1 / Ctrl+B, Ctrl+2 / Ctrl+J, M)
    if ((!mod && (key === "e" || key === "E")) || (mod && (key === "e" || key === "E"))) {
      event.preventDefault();
      if (game.capabilities.analyze.enabled) engine.setAnalysisEnabled(!engine.analysisEnabled);
      return;
    }

    if (mod && (key === "1" || key === "b" || key === "B")) {
      event.preventDefault();
      handlers.onToggleAnalysisPanel?.();
      return;
    }

    if (mod && (key === "2" || key === "j" || key === "J")) {
      event.preventDefault();
      handlers.onToggleMoveListPanel?.();
      return;
    }

    if (!mod && !shift && (key === "m" || key === "M")) {
      event.preventDefault();
      preferences.setSoundEnabled(!preferences.soundEnabled);
      return;
    }
  }

  onMounted(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onBeforeUnmount(() => {
    window.removeEventListener("keydown", handleKeydown);
  });
}
