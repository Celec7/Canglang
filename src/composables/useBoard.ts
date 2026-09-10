// 棋盘交互状态与辅助，由对局 store 驱动、并经 IPC 回落到 Rust 规则。负责棋子
// 选择、合法走法高亮，以及把走法分发给 store

import { computed, ref, watch } from "vue";
import { boardToView, viewToBoard } from "@/lib/board-view";
import {
  coordsToIccs,
  iccsToCoords,
  parseFen,
  pieceColor,
  type Board,
  type Coord,
  type Turn,
} from "@/lib/chess";
import { commands, unwrap } from "@/lib/ipc";
import { useGameStore } from "@/stores/game";
import { useEngineStore } from "@/stores/engine";
import { usePreferencesStore } from "@/stores/preferences";
import { useToast } from "@/composables/useToast";

function sameCoord(left: Coord | null, right: Coord): boolean {
  return !!left && left[0] === right[0] && left[1] === right[1];
}

export function useBoard(_options: { engineMoves?: boolean } = {}) {
  const game = useGameStore();
  const engine = useEngineStore();
  const preferences = usePreferencesStore();
  const { show } = useToast();
  const selected = ref<Coord | null>(null);
  const legalTargets = ref<Coord[]>([]);
  const focused = ref<Coord | null>(null);

  // 棋盘替换可能来自悔棋/重做、摆设局面或重放
  // 发生替换时清理临时选中状态和目标格
  watch(
    () => [game.fen, game.result],
    () => {
      selected.value = null;
      legalTargets.value = [];
    }
  );

  const lastMove = computed<{ from: Coord; to: Coord } | null>(() => {
    const list = game.appliedHistory;
    if (list.length === 0) return null;
    return iccsToCoords(list[list.length - 1].iccs);
  });

  const parsedPosition = computed(() => parseFen(game.fen));
  const board = computed<Board>(() => parsedPosition.value.board);
  const turn = computed<Turn>(() => parsedPosition.value.turn);
  const inCheck = computed(() => game.inCheck);
  const gameOver = computed(() => game.result !== "ongoing");
  const engineControlsTurn = computed(() => engine.isControlledTurn(turn.value));

  async function select(row: number, col: number) {
    if (gameOver.value) return;
    const piece = board.value[row]?.[col];
    if (!piece || pieceColor(piece) !== turn.value) return;

    try {
      const moves = await unwrap(await commands.getCandidateMoves(game.fen, row, col));
      const targets = moves
        .map((iccs) => iccsToCoords(iccs))
        .filter((move): move is { from: Coord; to: Coord } => !!move && sameCoord(move.from, [row, col]))
        .map((move) => move.to);
      selected.value = [row, col];
      legalTargets.value = targets;
    } catch (cause) {
      show(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function commitMove(from: Coord, to: Coord) {
    try {
      const result = await game.makeMove(coordsToIccs(from, to));
      if (result.legal) {
        selected.value = null;
        legalTargets.value = [];
        return;
      }

      show(inCheck.value ? "请应将" : "不可送将");
      selected.value = null;
      legalTargets.value = [];
    } catch (cause) {
      show(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function submit(from: Coord, to: Coord) {
    await commitMove(from, to);
  }

  async function onSquareClick(row: number, col: number) {
    if (gameOver.value) return;
    if (engineControlsTurn.value) {
      show("当前由引擎走棋");
      return;
    }

    const coord: Coord = [row, col];
    const piece = board.value[row]?.[col];

    if (!selected.value) {
    // NONE：只有当前行棋方的棋子可以开始交互；空格、敌方棋子和非当前回合保持无操作
      if (piece && pieceColor(piece) === turn.value) await select(row, col);
      return;
    }

    if (sameCoord(selected.value, coord)) return;

    if (piece && pieceColor(piece) === turn.value) {
      await select(row, col);
      return;
    }

    const isLegalTarget = legalTargets.value.some((target) => sameCoord(target, coord));
    const from = selected.value;
    if (isLegalTarget) {
      await submit(from, coord);
      return;
    }

    // 让 Rust 继续作为尝试走法的权威来源，不根据结果推断其它棋规
    // 此分支只在当前局面被将军时提供必要说明
    if (inCheck.value) {
      await submit(from, coord);
      return;
    }

    selected.value = null;
    legalTargets.value = [];
  }

  function clearSelection() {
    selected.value = null;
    legalTargets.value = [];
  }

  function setFocused(coord: Coord | null) {
    focused.value = coord;
  }

  function moveFocus(rowDelta: number, colDelta: number): Coord {
    const current = focused.value ?? (turn.value === "red" ? [9, 4] : [0, 4]);
    const view = boardToView(current, preferences.boardOrientation, turn.value);
    const nextView: Coord = [
      Math.min(9, Math.max(0, view[0] + rowDelta)),
      Math.min(8, Math.max(0, view[1] + colDelta)),
    ];
    const nextCoord = viewToBoard(nextView, preferences.boardOrientation, turn.value);
    focused.value = nextCoord;
    return nextCoord;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      clearSelection();
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      moveFocus(-1, 0);
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveFocus(1, 0);
      return;
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      moveFocus(0, -1);
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      moveFocus(0, 1);
      return;
    }
    if ((event.key === "Enter" || event.key === " ") && event.target === event.currentTarget) {
      event.preventDefault();
      if (focused.value) void onSquareClick(...focused.value);
    }
  }

  return {
    board,
    turn,
    inCheck,
    gameOver,
    selected,
    legalTargets,
    lastMove,
    focused,
    onSquareClick,
    clearSelection,
    setFocused,
    moveFocus,
    handleKeydown,
  };
}
