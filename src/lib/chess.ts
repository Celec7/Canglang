// 前端使用的中国象棋纯辅助：FEN <-> 棋盘网格、ICCS <-> 棋盘坐标、棋子字形/颜色
// 映射。所有规则校验都委托给 Rust 后端（经 IPC）；本模块只负责为渲染与点击处理
// 组织数据

export type Board = (string | null)[][];
export type Turn = "red" | "black";
export type Coord = [number, number]; // [row 0..9, col 0..8]

/** 归一化的棋盘视图状态模型（统一正常对局、局面编辑、PV 预览等场景） */
export interface BoardViewModel {
  board: Board;
  turn: Turn;
  inCheck?: boolean;
  lastMove?: { from: Coord; to: Coord } | null;
  selection?: Coord | null;
}

export const ROWS = 10;
export const COLS = 9;
export const INITIAL_FEN = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w";

// FEN 棋子字符 → 中文字形（大写表示红方）
export const PIECE_GLYPH: Record<string, string> = {
  K: "帥",
  A: "仕",
  B: "相",
  N: "馬",
  R: "車",
  C: "炮",
  P: "兵",
  k: "將",
  a: "士",
  b: "象",
  n: "馬",
  r: "車",
  c: "炮",
  p: "卒",
};

const JIEQI_PIECE_CODES: Record<string, [string, string]> = {
  king: ["K", "k"],
  advisor: ["A", "a"],
  bishop: ["B", "b"],
  knight: ["N", "n"],
  rook: ["R", "r"],
  cannon: ["C", "c"],
  pawn: ["P", "p"],
};

/** 将揭棋公开角色转换为对应阵营的中国象棋棋子名 */
export function jieqiPieceName(kind: string | null | undefined, color: Turn): string {
  if (!kind) return "未知棋子";
  const code = JIEQI_PIECE_CODES[kind]?.[color === "red" ? 0 : 1];
  return code ? PIECE_GLYPH[code] : "未知棋子";
}

export function pieceGlyph(ch: string): string {
  if (ch.startsWith("jieqi:hidden:")) return "";
  if (ch.startsWith("jieqi:revealed:")) {
    const [, , color, kind] = ch.split(":");
    return jieqiPieceName(kind, color === "black" ? "black" : "red");
  }
  return PIECE_GLYPH[ch] ?? ch;
}

export function pieceColor(ch: string): "red" | "black" {
  if (ch.startsWith("jieqi:")) return ch.split(":")[2] === "black" ? "black" : "red";
  return ch === ch.toUpperCase() ? "red" : "black";
}

/// 把 2 段式中国象棋 FEN 解析为 10x9 网格及走子方
export function parseFen(fen: string): { board: Board; turn: Turn } {
  const parts = fen.trim().split(/\s+/);
  const rows = (parts[0] ?? "").split("/");
  const board: Board = [];
  for (const row of rows) {
    const cells: (string | null)[] = [];
    for (const ch of row) {
      if (ch >= "1" && ch <= "9") {
        for (let i = 0; i < Number(ch); i++) cells.push(null);
      } else {
        cells.push(ch);
      }
    }
    while (cells.length < COLS) cells.push(null);
    board.push(cells);
  }
  const turn: Turn = parts[1] === "b" ? "black" : "red";
  return { board, turn };
}

/** 将编辑器棋盘序列化为跨边界使用的两段式中国象棋 FEN */
export function boardToFen(board: Board, turn: Turn): string {
  const rows = board.slice(0, ROWS).map((row) => {
    let empty = 0;
    let encoded = "";
    for (let col = 0; col < COLS; col++) {
      const piece = row[col] ?? null;
      if (piece) {
        if (empty > 0) encoded += empty;
        encoded += piece;
        empty = 0;
      } else {
        empty += 1;
      }
    }
    if (empty > 0) encoded += empty;
    return encoded || String(COLS);
  });
  return `${rows.join("/")} ${turn === "red" ? "w" : "b"}`;
}

export interface MoveCoords {
  from: Coord;
  to: Coord;
}

/// 把 4 字符 ICCS 走法解析为棋盘坐标，格式非法时返回 null
export function iccsToCoords(iccs: string): MoveCoords | null {
  if (iccs.length !== 4) return null;
  const fromCol = iccs.charCodeAt(0) - 97;
  const toCol = iccs.charCodeAt(2) - 97;
  const fromRow = 9 - Number(iccs[1]);
  const toRow = 9 - Number(iccs[3]);
  if (
    fromCol < 0 ||
    fromCol > 8 ||
    toCol < 0 ||
    toCol > 8 ||
    fromRow < 0 ||
    fromRow > 9 ||
    toRow < 0 ||
    toRow > 9
  ) {
    return null;
  }
  return { from: [fromRow, fromCol], to: [toRow, toCol] };
}

/// 把 `from`/`to` 棋盘坐标对编码为 4 字符 ICCS 字符串
export function coordsToIccs(from: Coord, to: Coord): string {
  return `${String.fromCharCode(97 + from[1])}${9 - from[0]}${String.fromCharCode(97 + to[1])}${
    9 - to[0]
  }`;
}
