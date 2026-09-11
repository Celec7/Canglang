import { jieqiPieceName, pieceColor, pieceGlyph, type Board } from "./chess";
import { RED_FILE_LABELS } from "./board-view";

const RANK_LABELS: readonly string[] = [
  "黑方底线", // 0
  "黑方底二线", // 1
  "黑方宫顶线", // 2
  "黑方卒林线", // 3
  "黑方河界线", // 4
  "红方河界线", // 5
  "红方兵林线", // 6
  "红方宫顶线", // 7
  "红方底二线", // 8
  "红方底线", // 9
];

/**
 * 获取棋盘指定横线的传统称谓
 */
export function traditionalRankName(row: number): string {
  return RANK_LABELS[row] ?? `第${row}线`;
}

/**
 * 获取棋盘指定坚路的传统路数称谓（红方右起一至九路，黑方右起1至9路）
 */
export function traditionalFileName(row: number, col: number): string {
  if (row >= 5) {
    return `${RED_FILE_LABELS[col]}路`;
  }
  return `${col + 1}路`;
}

export interface SquareLabelOptions {
  row: number;
  col: number;
  board: Board;
  isSelected?: boolean;
  isCandidateTarget?: boolean;
  isInCheck?: boolean;
}

/**
 * 生成符合中国传统象棋术语的交叉点无障碍文本
 */
export function formatTraditionalSquareLabel(options: SquareLabelOptions): string {
  const { row, col, board, isSelected, isCandidateTarget, isInCheck } = options;
  const piece = board[row]?.[col];
  const file = traditionalFileName(row, col);
  const rank = traditionalRankName(row);
  const posDescription = `${file}，${rank}`;

  let text = "";
  if (piece) {
    const color = pieceColor(piece);
    const side = color === "red" ? "红方" : "黑方";
    const jieqi = piece.split(":");
    const hiddenJieqi = jieqi[0] === "jieqi" && jieqi[1] === "hidden";
    const name = hiddenJieqi ? "暗子" : pieceGlyph(piece);
    text = `${side} ${name}，${posDescription}`;
    if (hiddenJieqi) {
      text += `，首步按${jieqiPieceName(jieqi[3], color)}行走`;
    }
  } else {
    text = `${posDescription}，空位`;
  }

  if (isSelected) {
    text += "，已选中";
  } else if (isCandidateTarget) {
    const candidate = board.some((rank) => rank.some((value) => value?.startsWith("jieqi:")))
      ? "候选"
      : "可";
    if (piece) {
      text += `，${candidate}吃子目标`;
    } else {
      text += `，${candidate}落子目标`;
    }
  }

  if (isInCheck && piece && (piece === "K" || piece === "k" || piece.endsWith(":king"))) {
    text += "，正被将军！";
  }

  return text;
}
