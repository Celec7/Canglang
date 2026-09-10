import { ref } from "vue";
import { boardToFen, INITIAL_FEN, parseFen, type Board, type Coord, type Turn } from "@/lib/chess";
import type { ChessManual } from "@/bindings";
import { commands, unwrap } from "@/lib/ipc";
import { useBookStore } from "@/stores/book";
import { useGameStore } from "@/stores/game";

export interface EditorPieceOption {
  code: string;
  label: string;
  side: "red" | "black";
}

export const RED_PIECES: EditorPieceOption[] = [
  { code: "K", label: "帅", side: "red" },
  { code: "A", label: "仕", side: "red" },
  { code: "B", label: "相", side: "red" },
  { code: "N", label: "马", side: "red" },
  { code: "R", label: "车", side: "red" },
  { code: "C", label: "炮", side: "red" },
  { code: "P", label: "兵", side: "red" },
];

export const BLACK_PIECES: EditorPieceOption[] = [
  { code: "k", label: "将", side: "black" },
  { code: "a", label: "士", side: "black" },
  { code: "b", label: "象", side: "black" },
  { code: "n", label: "马", side: "black" },
  { code: "r", label: "车", side: "black" },
  { code: "c", label: "炮", side: "black" },
  { code: "p", label: "卒", side: "black" },
];

export const EDITOR_PIECES: EditorPieceOption[] = [...RED_PIECES, ...BLACK_PIECES];

interface EditorSnapshot {
  board: Board;
  turn: Turn;
}

function cloneBoard(board: Board): Board {
  return board.map((row) => [...row]);
}

function emptyBoard(): Board {
  return Array.from({ length: 10 }, () => Array<string | null>(9).fill(null));
}

export function usePositionEditor() {
  const game = useGameStore();
  const book = useBookStore();
  const draftBoard = ref<Board>(emptyBoard());
  const turn = ref<Turn>("red");
  const selectedPiece = ref<string | null>(null);
  const selectedSquare = ref<Coord | null>(null);
  const pickedSquare = ref<Coord | null>(null);
  const undoStack = ref<EditorSnapshot[]>([]);
  const error = ref<string | null>(null);
  const validating = ref(false);
  const exportedPgn = ref("");

  function setDraft(snapshot: EditorSnapshot) {
    draftBoard.value = cloneBoard(snapshot.board);
    turn.value = snapshot.turn;
  }

  function loadFen(fen: string) {
    const parsed = parseFen(fen || INITIAL_FEN);
    setDraft({ board: parsed.board, turn: parsed.turn });
    undoStack.value = [];
    selectedSquare.value = null;
    pickedSquare.value = null;
    selectedPiece.value = null;
    error.value = null;
    exportedPgn.value = "";
  }

  function open() {
    if (!game.capabilities.edit_position.enabled || !game.fen) {
      error.value = "当前对局不支持局面编辑";
      return;
    }
    loadFen(game.fen);
  }

  function remember() {
    undoStack.value.push({ board: cloneBoard(draftBoard.value), turn: turn.value });
    if (undoStack.value.length > 32) undoStack.value.shift();
    error.value = null;
  }

  function selectPiece(piece: string | null) {
    // 点击当前已选中的棋子则取消提子，放回槽中
    if (selectedPiece.value === piece) {
      selectedPiece.value = null;
    } else {
      selectedPiece.value = piece;
    }
    pickedSquare.value = null;
  }

  function placeAt(coord: Coord) {
    handleSquareClick(coord);
  }

  function handleSquareClick(coord: Coord) {
    const [r, c] = coord;
    const existing = draftBoard.value[r]?.[c];

    // 1. 如果提了橡皮擦：单次擦除，用完手清空
    if (selectedPiece.value === "ERASE") {
      if (existing) {
        remember();
        draftBoard.value[r][c] = null;
      }
      selectedPiece.value = null;
      pickedSquare.value = null;
      return;
    }

    // 2. 如果从提子槽提了一颗棋子：单次放置，放完手立即清空
    if (selectedPiece.value !== null) {
      remember();
      draftBoard.value[r][c] = selectedPiece.value;
      selectedSquare.value = coord;
      selectedPiece.value = null; // 单次提一颗，放完归零
      return;
    }

    // 3. 如果此前已经在盘上拾起了一颗子：挪到目标格，挪完手清空
    if (pickedSquare.value) {
      const [fromR, fromC] = pickedSquare.value;
      if (fromR === r && fromC === c) {
        pickedSquare.value = null;
        return;
      }
      remember();
      const pieceToMove = draftBoard.value[fromR][fromC];
      draftBoard.value[fromR][fromC] = null;
      draftBoard.value[r][c] = pieceToMove;
      pickedSquare.value = null;
      selectedSquare.value = coord;
      return;
    }

    // 4. 当前手上无子：若点击了盘上已有棋子，将其拾起准备挪移
    if (existing) {
      pickedSquare.value = coord;
    }
  }

  function removeAt(coord: Coord) {
    const [r, c] = coord;
    if (draftBoard.value[r]?.[c]) {
      remember();
      draftBoard.value[r][c] = null;
      if (pickedSquare.value && pickedSquare.value[0] === r && pickedSquare.value[1] === c) {
        pickedSquare.value = null;
      }
    }
  }

  function clearBoard() {
    remember();
    draftBoard.value = emptyBoard();
    selectedSquare.value = null;
    pickedSquare.value = null;
    selectedPiece.value = null;
  }

  function resetInitial() {
    loadFen(INITIAL_FEN);
  }

  function setTurn(nextTurn: Turn) {
    if (turn.value === nextTurn) return;
    remember();
    turn.value = nextTurn;
  }

  function undo() {
    const previous = undoStack.value.pop();
    if (previous) setDraft(previous);
    selectedSquare.value = null;
    error.value = null;
  }

  async function confirm(): Promise<boolean> {
    validating.value = true;
    error.value = null;
    try {
      const fen = boardToFen(draftBoard.value, turn.value);
      const normalized = await unwrap(await commands.validatePosition(fen));
      if (!(await game.newGame(normalized))) return false;
      book.clearQuery();
      return true;
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      return false;
    } finally {
      validating.value = false;
    }
  }

  async function importText(format: "fen" | "pgn", text: string): Promise<boolean> {
    error.value = null;
    try {
      const source = text.trim();
      if (!source) throw new Error("请输入要导入的内容");
      const startFen = format === "fen"
        ? await unwrap(await commands.parseFen(source))
        : (await unwrap(await commands.manualParseText(format, source))).start_fen;
      loadFen(startFen);
      return true;
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      return false;
    }
  }

  async function importFile(path: string): Promise<boolean> {
    error.value = null;
    try {
      const manual = await unwrap(await commands.manualLoad(path.trim()));
      loadFen(manual.start_fen || INITIAL_FEN);
      return true;
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      return false;
    }
  }

  async function exportPgn(): Promise<string> {
    const root: ChessManual["root"] = {
      id: 0,
      mv: null,
      chinese_notation: "开始局面",
      comment: null,
      score: null,
      children: [],
    };
    const manual: ChessManual = {
      title: "自定义局面",
      date: null,
      red_player: null,
      black_player: null,
      event_name: null,
      start_fen: boardToFen(draftBoard.value, turn.value),
      root,
    };
    exportedPgn.value = await commands.manualExportPgn(manual);
    return exportedPgn.value;
  }

  return {
    draftBoard,
    turn,
    selectedPiece,
    selectedSquare,
    pickedSquare,
    undoStack,
    error,
    validating,
    exportedPgn,
    open,
    selectPiece,
    placeAt,
    handleSquareClick,
    removeAt,
    clearBoard,
    resetInitial,
    setTurn,
    undo,
    confirm,
    importText,
    importFile,
    exportPgn,
  };
}
