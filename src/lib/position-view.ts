import type { JieqiPieceView, JieqiPublicKind, PositionView } from "@/bindings";
import { parseFen, type Board, type Turn } from "@/lib/chess";

const HIDDEN_PREFIX = "jieqi:hidden";
const REVEALED_PREFIX = "jieqi:revealed";

export function jieqiPieceToken(piece: JieqiPieceView): string {
  const kind = piece.state === "hidden" ? piece.move_as : piece.kind;
  const state = piece.state === "hidden" ? HIDDEN_PREFIX : REVEALED_PREFIX;
  return `${state}:${piece.color}:${kind}`;
}

export function isJieqiPieceToken(piece: string): boolean {
  return piece.startsWith("jieqi:");
}

export function parseJieqiPieceToken(piece: string): {
  state: "hidden" | "revealed";
  color: Turn;
  kind: JieqiPublicKind;
} | null {
  const [namespace, state, color, kind] = piece.split(":");
  if (
    namespace !== "jieqi" ||
    (state !== "hidden" && state !== "revealed") ||
    (color !== "red" && color !== "black") ||
    !["king", "advisor", "bishop", "knight", "rook", "cannon", "pawn"].includes(kind)
  ) {
    return null;
  }
  return { state, color, kind: kind as JieqiPublicKind };
}

export function positionViewToBoard(position: PositionView): { board: Board; turn: Turn } {
  if (position.variant === "xiangqi") return parseFen(position.fen);

  const board: Board = Array.from({ length: 10 }, () => Array<string | null>(9).fill(null));
  for (const piece of position.position.pieces) {
    const { row, col } = piece.position;
    if (row >= 0 && row < 10 && col >= 0 && col < 9) {
      board[row][col] = jieqiPieceToken(piece);
    }
  }
  return { board, turn: position.position.turn };
}
