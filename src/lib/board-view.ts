import type { BoardOrientation } from "@/lib/preferences";
import type { Coord, Turn } from "@/lib/chess";

export const CELL = 60;
export const MARGIN = 40;
export const BOARD_COLUMNS = 9;
export const BOARD_ROWS = 10;
export const BOARD_WIDTH = (BOARD_COLUMNS - 1) * CELL + 2 * MARGIN;
export const BOARD_HEIGHT = (BOARD_ROWS - 1) * CELL + 2 * MARGIN;

export interface Pixel {
  x: number;
  y: number;
}

export interface BoardGeometry {
  cell: number;
  margin: number;
  width: number;
  height: number;
  pieceRadius: number;
  pieceFaceRadius: number;
  selectionOuter: number;
  selectionInner: number;
  boardMarkOuter: number;
  boardMarkInner: number;
  hintRadius: number;
  boardRadius: number;
  boardInset: number;
  innerFrameRadius: number;
  frameStroke: number;
  gridStroke: number;
  markStroke: number;
  pieceFontSize: number;
  riverFontSize: number;
  riverLetterSpacing: number;
  coordinateFontSize: number;
  coordinateOffset: number;
  checkRadius: number;
  checkStroke: number;
  hitRadius: number;
}

export const BOARD_GEOMETRY: BoardGeometry = {
  cell: CELL,
  margin: MARGIN,
  width: BOARD_WIDTH,
  height: BOARD_HEIGHT,
  pieceRadius: CELL * 0.4,
  pieceFaceRadius: CELL / 3,
  // 选框只比棋子外沿多约 4px，避免形成第二层“大圆环”
  selectionOuter: CELL * 0.4 + 4,
  selectionInner: CELL * 0.4 - 4,
  boardMarkOuter: CELL * 0.17,
  boardMarkInner: CELL * 0.085,
  hintRadius: CELL * 0.10,
  boardRadius: CELL * 0.233,
  boardInset: CELL * 0.2,
  innerFrameRadius: CELL * 0.15,
  frameStroke: CELL / 15,
  gridStroke: CELL / 30,
  markStroke: CELL / 33,
  pieceFontSize: CELL * 0.4,
  riverFontSize: CELL * 0.5,
  riverLetterSpacing: CELL * 0.133,
  coordinateFontSize: CELL * 0.2,
  coordinateOffset: CELL * 0.233,
  checkRadius: CELL * 0.45,
  checkStroke: CELL / 15,
  hitRadius: CELL * 0.466,
};

export const RED_FILE_LABELS = ["九", "八", "七", "六", "五", "四", "三", "二", "一"];

export type ResolvedBoardOrientation = "red" | "black";

/** 将用户偏好解析为当前局面使用的视角 */
export function resolveBoardOrientation(
  orientation: BoardOrientation,
  turn: Turn
): ResolvedBoardOrientation {
  return orientation === "follow_turn" ? turn : orientation;
}

/** 将逻辑棋盘坐标转换为屏幕上的行列坐标 */
export function boardToView(coord: Coord, orientation: BoardOrientation, turn: Turn): Coord {
  if (resolveBoardOrientation(orientation, turn) === "red") return coord;
  return [BOARD_ROWS - 1 - coord[0], BOARD_COLUMNS - 1 - coord[1]];
}

/** 将屏幕上的行列坐标转换回逻辑棋盘坐标 */
export function viewToBoard(coord: Coord, orientation: BoardOrientation, turn: Turn): Coord {
  return boardToView(coord, orientation, turn);
}

/** 预先计算的 10x9 屏幕网格像素坐标缓存表（O(1) 瞬时查询） */
export const VIEW_PIXELS: Pixel[][] = (() => {
  const grid: Pixel[][] = [];
  for (let r = 0; r < BOARD_ROWS; r++) {
    const row: Pixel[] = [];
    for (let c = 0; c < BOARD_COLUMNS; c++) {
      row.push({
        x: MARGIN + c * CELL,
        y: MARGIN + r * CELL,
      });
    }
    grid.push(row);
  }
  return grid;
})();

/** 将视图坐标转换为 SVG 像素位置 */
export function viewToPixel(coord: Coord): Pixel {
  const row = VIEW_PIXELS[coord[0]];
  if (row && row[coord[1]]) return row[coord[1]];
  return {
    x: MARGIN + coord[1] * CELL,
    y: MARGIN + coord[0] * CELL,
  };
}

/**
 * 预先烘焙合并的中国象棋全量静态几何网格路径（包含 10 横线、16 竖线、4 九宫斜线、14 处兵炮位折角）
 * 由于中国象棋棋盘几何 180 度中心对称，红黑视角屏幕像素路径完全一致，可作为编译期常数直接复用
 */
export const STATIC_GRID_PATH: string = (() => {
  const parts: string[] = [];

  // 1. 10 条横线
  for (let r = 0; r < 10; r++) {
    const p1 = VIEW_PIXELS[r][0];
    const p2 = VIEW_PIXELS[r][8];
    parts.push(`M ${p1.x} ${p1.y} L ${p2.x} ${p2.y}`);
  }

  // 2. 左右 2 条贯通外侧竖线
  const tl = VIEW_PIXELS[0][0];
  const bl = VIEW_PIXELS[9][0];
  parts.push(`M ${tl.x} ${tl.y} L ${bl.x} ${bl.y}`);
  const tr = VIEW_PIXELS[0][8];
  const br = VIEW_PIXELS[9][8];
  parts.push(`M ${tr.x} ${tr.y} L ${br.x} ${br.y}`);

  // 3. 中间 7 条被楚河汉界隔断的竖线
  for (let c = 1; c <= 7; c++) {
    const topStart = VIEW_PIXELS[0][c];
    const topEnd = VIEW_PIXELS[4][c];
    parts.push(`M ${topStart.x} ${topStart.y} L ${topEnd.x} ${topEnd.y}`);

    const bottomStart = VIEW_PIXELS[5][c];
    const bottomEnd = VIEW_PIXELS[9][c];
    parts.push(`M ${bottomStart.x} ${bottomStart.y} L ${bottomEnd.x} ${bottomEnd.y}`);
  }

  // 4. 上下九宫格 4 条对角斜线
  const p03 = VIEW_PIXELS[0][3];
  const p25 = VIEW_PIXELS[2][5];
  parts.push(`M ${p03.x} ${p03.y} L ${p25.x} ${p25.y}`);
  const p05 = VIEW_PIXELS[0][5];
  const p23 = VIEW_PIXELS[2][3];
  parts.push(`M ${p05.x} ${p05.y} L ${p23.x} ${p23.y}`);

  const p73 = VIEW_PIXELS[7][3];
  const p95 = VIEW_PIXELS[9][5];
  parts.push(`M ${p73.x} ${p73.y} L ${p95.x} ${p95.y}`);
  const p75 = VIEW_PIXELS[7][5];
  const p93 = VIEW_PIXELS[9][3];
  parts.push(`M ${p75.x} ${p75.y} L ${p93.x} ${p93.y}`);

  // 5. 14 处兵/炮位折角标记
  const markCoords: Coord[] = [
    [2, 1], [2, 7],
    [3, 0], [3, 2], [3, 4], [3, 6], [3, 8],
    [6, 0], [6, 2], [6, 4], [6, 6], [6, 8],
    [7, 1], [7, 7],
  ];

  const { boardMarkInner: inner, boardMarkOuter: outer } = BOARD_GEOMETRY;

  for (const [r, c] of markCoords) {
    const center = VIEW_PIXELS[r][c];
    const addBracket = (hDir: -1 | 1, vDir: -1 | 1) => {
      const xInner = center.x + hDir * inner;
      const xOuter = center.x + hDir * outer;
      const yInner = center.y + vDir * inner;
      const yOuter = center.y + vDir * outer;
      // 兵炮位折角：以紧贴十字交叉线为直角顶点，臂朝外展开（凹形折角）
      parts.push(`M ${xOuter} ${yInner} H ${xInner} V ${yOuter}`);
    };

    if (c > 0) {
      addBracket(-1, -1);
      addBracket(-1, 1);
    }
    if (c < 8) {
      addBracket(1, -1);
      addBracket(1, 1);
    }
  }

  return parts.join(" ");
})();

/** 将逻辑棋盘坐标直接转换为 SVG 像素位置 */
export function boardToPixel(coord: Coord, orientation: BoardOrientation, turn: Turn): Pixel {
  return viewToPixel(boardToView(coord, orientation, turn));
}

/** 将指针位置解析为最近的视图交叉点 */
export function pixelToView(pixel: Pixel, tolerance = CELL / 2): Coord | null {
  const col = Math.round((pixel.x - MARGIN) / CELL);
  const row = Math.round((pixel.y - MARGIN) / CELL);
  if (row < 0 || row >= BOARD_ROWS || col < 0 || col >= BOARD_COLUMNS) return null;

  const target = viewToPixel([row, col]);
  if (Math.hypot(pixel.x - target.x, pixel.y - target.y) > tolerance) return null;
  return [row, col];
}

/** 创建用于选中标记和上一步标记的四个开放折角 */
export function cornerPaths(
  center: Pixel,
  outer = BOARD_GEOMETRY.selectionOuter,
  inner = BOARD_GEOMETRY.selectionInner
): string[] {
  return [
    `M ${center.x - inner} ${center.y - outer} H ${center.x - outer} V ${center.y - inner}`,
    `M ${center.x + inner} ${center.y - outer} H ${center.x + outer} V ${center.y - inner}`,
    `M ${center.x - inner} ${center.y + outer} H ${center.x - outer} V ${center.y + inner}`,
    `M ${center.x + inner} ${center.y + outer} H ${center.x + outer} V ${center.y + inner}`,
  ];
}

/** 创建棋盘交叉点标记周围紧凑且独立的折角 */
export function boardMarkPaths(
  coord: Coord,
  orientation: BoardOrientation,
  turn: Turn
): string[] {
  const view = boardToView(coord, orientation, turn);
  const center = viewToPixel(view);
  const { boardMarkInner: inner, boardMarkOuter: outer } = BOARD_GEOMETRY;
  const paths: string[] = [];

  function addBracket(horizontalDirection: -1 | 1, verticalDirection: -1 | 1) {
    const xInner = center.x + horizontalDirection * inner;
    const xOuter = center.x + horizontalDirection * outer;
    const yInner = center.y + verticalDirection * inner;
    const yOuter = center.y + verticalDirection * outer;
    // 兵炮位折角：以紧贴十字交叉线为直角顶点，臂朝外展开（凹形折角）
    paths.push(`M ${xOuter} ${yInner} H ${xInner} V ${yOuter}`);
  }

  if (view[1] > 0) {
    addBracket(-1, -1);
    addBracket(-1, 1);
  }
  if (view[1] < BOARD_COLUMNS - 1) {
    addBracket(1, -1);
    addBracket(1, 1);
  }
  return paths;
}

/** 将交叉点转换为稳定且面向屏幕的 SVG 平移值 */
export function boardTransform(coord: Coord, orientation: BoardOrientation, turn: Turn): string {
  const point = boardToPixel(coord, orientation, turn);
  return `translate(${point.x}, ${point.y})`;
}

/** 标记面向屏幕的路数，同时保留双方的传统记谱方式 */
export function fileLabelAtView(
  viewRow: number,
  viewCol: number,
  orientation: BoardOrientation,
  turn: Turn
): string {
  const [logicalRow, logicalCol] = viewToBoard([viewRow, viewCol], orientation, turn);
  return logicalRow >= 5 ? RED_FILE_LABELS[logicalCol] : String(logicalCol + 1);
}

// 兼容现有调用方，等待棋盘迁移到 logical → view → pixel 命名
export const boardToViewCoord = boardToView;
export const viewToBoardCoord = viewToBoard;

export interface PieceMotion {
  piece: string;
  from: Coord;
  to: Coord;
}

/**
 * 对比两个棋盘局面，计算出所有移动棋子的位移路径（支持单步前进、单步悔棋及多步倒流）
 * 被吃掉复活的棋子不生成 motion，留在目标格原地复原
 */
export function computeBoardMotions(
  oldBoard: (string | null)[][],
  newBoard: (string | null)[][],
  hintLastMove?: { from: Coord; to: Coord } | null
): PieceMotion[] {
  // 若提供了确切的单步走法提示且新旧棋盘刚好满足单步位移，优先直接采用单步走法
  if (hintLastMove) {
    const fromPiece = oldBoard[hintLastMove.from[0]]?.[hintLastMove.from[1]];
    const toPiece = newBoard[hintLastMove.to[0]]?.[hintLastMove.to[1]];
    if (fromPiece && toPiece && fromPiece === toPiece) {
      return [{ piece: fromPiece, from: hintLastMove.from, to: hintLastMove.to }];
    }
  }

  const motions: PieceMotion[] = [];
  const pieceKinds = [...new Set([...oldBoard.flat(), ...newBoard.flat()].filter((piece): piece is string => piece !== null))];

  for (const p of pieceKinds) {
    const fromSquares: Coord[] = [];
    const toSquares: Coord[] = [];

    for (let r = 0; r < BOARD_ROWS; r++) {
      for (let c = 0; c < BOARD_COLUMNS; c++) {
        const oldP = oldBoard[r]?.[c] ?? null;
        const newP = newBoard[r]?.[c] ?? null;
        if (oldP === p && newP !== p) {
          fromSquares.push([r, c]);
        }
        if (newP === p && oldP !== p) {
          toSquares.push([r, c]);
        }
      }
    }

    // 将离开位置与到达位置按最短欧氏距离配对（最近优先配对，符合视觉运动直觉）
    while (fromSquares.length > 0 && toSquares.length > 0) {
      let bestDist = Infinity;
      let bestFromIdx = 0;
      let bestToIdx = 0;

      for (let f = 0; f < fromSquares.length; f++) {
        for (let t = 0; t < toSquares.length; t++) {
          const dr = fromSquares[f][0] - toSquares[t][0];
          const dc = fromSquares[f][1] - toSquares[t][1];
          const dist = dr * dr + dc * dc;
          if (dist < bestDist) {
            bestDist = dist;
            bestFromIdx = f;
            bestToIdx = t;
          }
        }
      }

      motions.push({
        piece: p,
        from: fromSquares[bestFromIdx],
        to: toSquares[bestToIdx],
      });
      fromSquares.splice(bestFromIdx, 1);
      toSquares.splice(bestToIdx, 1);
    }
  }

  return motions;
}
