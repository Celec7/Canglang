<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useBoard } from "@/composables/useBoard";
import { formatTraditionalSquareLabel } from "@/lib/a11y";
import {
  BOARD_GEOMETRY,
  BOARD_HEIGHT,
  BOARD_WIDTH,
  boardToPixel,
  boardTransform,
  computeBoardMotions,
  cornerPaths as makeCornerPaths,
  fileLabelAtView,
  STATIC_GRID_PATH,
  VIEW_PIXELS,
} from "@/lib/board-view";
import {
  iccsToCoords,
  pieceColor,
  pieceGlyph,
  type Board,
  type BoardViewModel,
  type Coord,
  type Turn,
} from "@/lib/chess";
import { CHESS_MOVE_EASE } from "@/lib/motion";
import { useEngineStore } from "@/stores/engine";
import { usePreferencesStore } from "@/stores/preferences";

const props = withDefaults(
  defineProps<{
    /** 归一化视图模型（优先使用，解耦多场景分支） */
    view?: BoardViewModel | null;
    editorBoard?: Board;
    editorTurn?: Turn;
    editorMode?: boolean;
    editorSelectedSquare?: Coord | null;
    previewBoard?: Board;
    previewTurn?: Turn;
    previewInCheck?: boolean;
    previewLastMove?: { from: Coord; to: Coord } | null;
    previewing?: boolean;
  }>(),
  { editorMode: false }
);
const emit = defineEmits<{
  squareClick: [coord: Coord];
  squareRightClick: [coord: Coord];
}>();

const preferences = usePreferencesStore();
const engine = useEngineStore();
const {
  board,
  turn,
  inCheck,
  selected,
  legalTargets,
  lastMove,
  focused,
  onSquareClick,
  setFocused,
  moveFocus,
  clearSelection,
} = useBoard({ engineMoves: !props.editorMode });

const renderedBoard = computed(() => props.view?.board ?? props.previewBoard ?? props.editorBoard ?? board.value);
const renderedTurn = computed(() => props.view?.turn ?? props.previewTurn ?? props.editorTurn ?? turn.value);
const renderedInCheck = computed(() => props.view?.inCheck ?? props.previewInCheck ?? inCheck.value);
const renderedLastMove = computed(() => (props.view ? props.view.lastMove ?? null : props.previewLastMove ?? lastMove.value));
const renderedSelection = computed(() => {
  if (props.view) return props.view.selection ?? null;
  return props.editorMode ? props.editorSelectedSquare ?? null : selected.value;
});
const keyboardFocus = ref(false);
const activeSquare = ref<Coord>([9, 4]);

watch(
  () => lastMove.value,
  (mv) => {
    if (mv) {
      activeSquare.value = mv.to;
    }
  }
);

defineExpose({
  selected: renderedSelection,
});

function cloneBoard(b: Board): Board {
  return b.map((r) => [...r]);
}

const visualBoard = ref<Board>(cloneBoard(renderedBoard.value));

interface ActiveMotion {
  id: number;
  piece: string;
  fromX: number;
  fromY: number;
  toX: number;
  toY: number;
  toCoord: Coord;
}

const activeMotions = ref<ActiveMotion[]>([]);
const motionEls = new Map<number, SVGElement>();

function setMotionRef(id: number, el: SVGElement | null) {
  if (el) motionEls.set(id, el);
  else motionEls.delete(id);
}

let rafId: number | null = null;

function cancelAnimation() {
  if (rafId !== null) {
    cancelAnimationFrame(rafId);
    rafId = null;
  }
  activeMotions.value = [];
  motionEls.clear();
}

interface HitTargetSquare {
  row: number;
  col: number;
  x: number;
  y: number;
}

const hitTargets = computed<HitTargetSquare[]>(() => {
  const list: HitTargetSquare[] = [];
  for (let r = 0; r < 10; r++) {
    for (let c = 0; c < 9; c++) {
      const pt = point(r, c);
      list.push({ row: r, col: c, x: pt.x, y: pt.y });
    }
  }
  return list;
});

const coordinateLabels = computed(() => {
  const top: Array<{ x: number; label: string }> = [];
  const bottom: Array<{ x: number; label: string }> = [];
  for (let c = 0; c < 9; c++) {
    top.push({
      x: VIEW_PIXELS[0][c].x,
      label: fileLabelAtView(0, c, preferences.boardOrientation, renderedTurn.value),
    });
    bottom.push({
      x: VIEW_PIXELS[9][c].x,
      label: fileLabelAtView(9, c, preferences.boardOrientation, renderedTurn.value),
    });
  }
  return { top, bottom };
});

const riverCenterY = (VIEW_PIXELS[4][4].y + VIEW_PIXELS[5][4].y) / 2;

interface EngineBoardArrow {
  id: string;
  from: Coord;
  to: Coord;
  opacity: number;
  color: string;
  path: string;
}

interface EngineArrowStyle {
  opacity: number;
  shaft: number;
  headW: number;
  headL: number;
  color: string;
}

function calcArrowPolygon(
  from: Coord,
  to: Coord,
  shaftWidth: number,
  headWidth: number,
  headLength: number
): string {
  const p1 = point(from[0], from[1]);
  const p2 = point(to[0], to[1]);
  const dx = p2.x - p1.x;
  const dy = p2.y - p1.y;
  const dist = Math.hypot(dx, dy);
  if (dist < 10) return "";

  const ux = dx / dist;
  const uy = dy / dist;
  const nx = -uy;
  const ny = ux;

  const startOffset = Math.min(22, dist * 0.25);
  const endOffset = Math.min(18, dist * 0.2);

  const effHl = Math.min(headLength, (dist - startOffset - endOffset) * 0.55);
  const effHw = headWidth * (effHl / headLength);

  const startX = p1.x + ux * startOffset;
  const startY = p1.y + uy * startOffset;

  const tipX = p2.x - ux * endOffset;
  const tipY = p2.y - uy * endOffset;

  const baseEndX = tipX - ux * effHl;
  const baseEndY = tipY - uy * effHl;

  const halfShaft = shaftWidth / 2;
  const halfHead = effHw / 2;

  const p0x = startX + nx * halfShaft;
  const p0y = startY + ny * halfShaft;
  const p1x = baseEndX + nx * halfShaft;
  const p1y = baseEndY + ny * halfShaft;
  const p2x = baseEndX + nx * halfHead;
  const p2y = baseEndY + ny * halfHead;
  const p3x = tipX;
  const p3y = tipY;
  const p4x = baseEndX - nx * halfHead;
  const p4y = baseEndY - ny * halfHead;
  const p5x = baseEndX - nx * halfShaft;
  const p5y = baseEndY - ny * halfShaft;
  const p6x = startX - nx * halfShaft;
  const p6y = startY - ny * halfShaft;

  return `M ${p0x.toFixed(1)} ${p0y.toFixed(1)} ` +
    `L ${p1x.toFixed(1)} ${p1y.toFixed(1)} ` +
    `L ${p2x.toFixed(1)} ${p2y.toFixed(1)} ` +
    `L ${p3x.toFixed(1)} ${p3y.toFixed(1)} ` +
    `L ${p4x.toFixed(1)} ${p4y.toFixed(1)} ` +
    `L ${p5x.toFixed(1)} ${p5y.toFixed(1)} ` +
    `L ${p6x.toFixed(1)} ${p6y.toFixed(1)} Z`;
}

function makeEngineArrow(id: string, iccs: string | undefined, style: EngineArrowStyle): EngineBoardArrow | null {
  if (!iccs) return null;
  const coords = iccsToCoords(iccs);
  if (!coords) return null;
  const path = calcArrowPolygon(coords.from, coords.to, style.shaft, style.headW, style.headL);
  if (!path) return null;
  return { id, from: coords.from, to: coords.to, opacity: style.opacity, color: style.color, path };
}

const engineArrows = computed<EngineBoardArrow[]>(() => {
  if (props.editorMode || !preferences.showEngineArrow || !engine.analysisEnabled) return [];

  const list = engine.multiPvList;
  const opponentStyle: EngineArrowStyle = {
    opacity: 0.75,
    shaft: 6.5,
    headW: 19,
    headL: 17,
    color: "#dc2626",
  };

  if (list.length === 0) {
    const pv = engine.latestThink?.pv;
    const primary = makeEngineArrow("primary", pv?.[0], {
      opacity: 0.75,
      shaft: 7,
      headW: 20,
      headL: 18,
      color: "#2563eb",
    });
    const opponent = makeEngineArrow("opponent", pv?.[1], opponentStyle);
    return [primary, opponent].filter((arrow): arrow is EngineBoardArrow => arrow !== null);
  }

  const styles = [
    { opacity: 0.75, shaft: 7.5, headW: 21, headL: 19, color: "#2563eb" },
    { opacity: 0.75, shaft: 5.5, headW: 16, headL: 15, color: "#0284c7" },
    { opacity: 0.75, shaft: 4, headW: 13, headL: 12, color: "#0d9488" },
    { opacity: 0.75, shaft: 3, headW: 10, headL: 10, color: "#64748b" },
  ];

  const result: EngineBoardArrow[] = [];
  for (let i = 0; i < list.length; i++) {
    const item = list[i];
    const style = styles[i] ?? styles[styles.length - 1];
    const arrow = makeEngineArrow(`pv-${item.multi_pv}`, item.pv?.[0], style);
    if (arrow) result.push(arrow);
  }

  const opponent = makeEngineArrow("opponent", list[0]?.pv?.[1], opponentStyle);
  if (opponent) result.push(opponent);
  return result;
});

function point(row: number, col: number) {
  return boardToPixel([row, col], preferences.boardOrientation, renderedTurn.value);
}

function reducedMotion(): boolean {
  return typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

watch(
  renderedBoard,
  (newBoard) => {
    const oldBoard = visualBoard.value;
    cancelAnimation();

    if (reducedMotion() || !preferences.animations) {
      visualBoard.value = cloneBoard(newBoard);
      return;
    }

    const duration = preferences.moveAnimationSeconds * 1000;
    if (duration <= 0) {
      visualBoard.value = cloneBoard(newBoard);
      return;
    }

    const motions = computeBoardMotions(oldBoard, newBoard, lastMove.value);
    if (motions.length === 0) {
      visualBoard.value = cloneBoard(newBoard);
      return;
    }

    // 过渡局面：目的地在运动期间置空（彻底杜绝目标格提前闪现导致的回弹），复活的棋子立即在目标格呈现
    const transientBoard = cloneBoard(newBoard);
    for (const m of motions) {
      transientBoard[m.to[0]][m.to[1]] = null;
    }
    visualBoard.value = transientBoard;

    activeMotions.value = motions.map((m, idx) => {
      const fromP = point(m.from[0], m.from[1]);
      const toP = point(m.to[0], m.to[1]);
      return {
        id: idx,
        piece: m.piece,
        fromX: fromP.x,
        fromY: fromP.y,
        toX: toP.x,
        toY: toP.y,
        toCoord: m.to,
      };
    });

    const motionData = activeMotions.value;
    let startTime: number | null = null;
    let lastFrameTime: number | null = null;
    let frameCount = 0;
    let maxFrameGapMs = 0;

    const tick = (now: number) => {
      if (startTime === null) {
        startTime = now;
        lastFrameTime = now;
      } else if (lastFrameTime !== null) {
        const gap = now - lastFrameTime;
        if (gap > maxFrameGapMs) maxFrameGapMs = gap;
        lastFrameTime = now;
      }
      frameCount++;

      const elapsed = now - startTime;
      const progress = Math.min(1, elapsed / duration);
      const eased = CHESS_MOVE_EASE(progress);

      for (const m of motionData) {
        const curX = m.fromX + (m.toX - m.fromX) * eased;
        const curY = m.fromY + (m.toY - m.fromY) * eased;
        const el = motionEls.get(m.id);
        if (el) {
          el.setAttribute("transform", `translate(${curX}, ${curY})`);
        }
      }

      if (progress < 1) {
        rafId = requestAnimationFrame(tick);
      } else {
        rafId = null;
        const totalDuration = now - startTime;
        const fps = Math.round((frameCount / (totalDuration / 1000)) * 10) / 10;
        if (import.meta.env.DEV) {
          console.debug(
            `[Motion Telemetry] ${totalDuration.toFixed(1)}ms | ${frameCount} frames | ${fps} FPS | maxFrameGap: ${maxFrameGapMs.toFixed(1)}ms`
          );
        }
        cancelAnimation();
        visualBoard.value = cloneBoard(newBoard);
      }
    };

    rafId = requestAnimationFrame(tick);
  },
  { deep: true }
);

watch(
  () => preferences.boardOrientation,
  () => {
    cancelAnimation();
    visualBoard.value = cloneBoard(renderedBoard.value);
  }
);

onBeforeUnmount(cancelAnimation);

function onBoardSquareClick(row: number, col: number) {
  if (props.previewing) return;
  const coord: Coord = [row, col];
  if (props.editorMode) {
    emit("squareClick", coord);
  } else {
    void onSquareClick(...coord);
  }
}

function onBoardSquareRightClick(row: number, col: number) {
  const coord: Coord = [row, col];
  if (props.editorMode) {
    emit("squareRightClick", coord);
  }
}

const squareRefs = new Map<string, SVGCircleElement>();
function setSquareRef(row: number, col: number, el: unknown) {
  const key = `${row},${col}`;
  if (el) {
    squareRefs.set(key, el as SVGCircleElement);
  } else {
    squareRefs.delete(key);
  }
}

function focusSquareDom(row: number, col: number) {
  const el = squareRefs.get(`${row},${col}`);
  el?.focus();
}

function onBoardSquareFocus(row: number, col: number) {
  activeSquare.value = [row, col];
  setFocused([row, col]);
}

function onBoardSquareBlur() {
  setFocused(null);
}

function onSquareKeydown(event: KeyboardEvent, row: number, col: number) {
  if (event.key.startsWith("Arrow")) {
    event.preventDefault();
    keyboardFocus.value = true;
    let dRow = 0;
    let dCol = 0;
    if (event.key === "ArrowUp") dRow = -1;
    else if (event.key === "ArrowDown") dRow = 1;
    else if (event.key === "ArrowLeft") dCol = -1;
    else if (event.key === "ArrowRight") dCol = 1;

    const nextCoord = moveFocus(dRow, dCol);
    if (nextCoord) {
      activeSquare.value = nextCoord;
      focusSquareDom(nextCoord[0], nextCoord[1]);
    }
    return;
  }

  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    keyboardFocus.value = true;
    onBoardSquareClick(row, col);
    return;
  }

  if (event.key === "Escape") {
    event.preventDefault();
    keyboardFocus.value = true;
    clearSelection();
    return;
  }
}

function onBoardSquarePointerDown() {
  keyboardFocus.value = false;
}

const squareLabels = computed<string[]>(() => {
  const currentBoard = renderedBoard.value;
  const sel = renderedSelection.value;
  const targets = legalTargets.value;
  const targetSet = new Set(targets.map(([r, c]) => r * 9 + c));
  const inCheckVal = renderedInCheck.value;
  const turnVal = renderedTurn.value;

  const labels: string[] = new Array(90);
  for (let r = 0; r < 10; r++) {
    for (let c = 0; c < 9; c++) {
      const idx = r * 9 + c;
      const isSel = sel ? sel[0] === r && sel[1] === c : false;
      const isTarget = targetSet.has(idx);
      const targetPiece = currentBoard[r]?.[c];
      const isKingCheck =
        inCheckVal &&
        !!targetPiece &&
        isKing(targetPiece) &&
        pieceColor(targetPiece) === turnVal;

      labels[idx] = formatTraditionalSquareLabel({
        row: r,
        col: c,
        board: currentBoard,
        isSelected: isSel,
        isLegalTarget: isTarget,
        isInCheck: isKingCheck,
      });
    }
  }
  return labels;
});

function isKing(piece: string): boolean {
  return piece === "K" || piece === "k" || piece.endsWith(":king");
}

function shouldShowFocusMark(): boolean {
  if (!keyboardFocus.value) return false;
  const target = focused.value ?? activeSquare.value;
  if (!target) return false;
  if (!renderedSelection.value) return true;
  return target[0] !== renderedSelection.value[0] || target[1] !== renderedSelection.value[1];
}
</script>

<template>
  <svg
    :viewBox="`0 0 ${BOARD_WIDTH} ${BOARD_HEIGHT}`"
    class="h-full w-full select-none outline-none"
    role="region"
    aria-label="中国象棋棋盘"
  >
    <defs>
      <linearGradient id="board-wood" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="var(--board-surface-light)" />
        <stop offset="0.52" stop-color="var(--board-surface)" />
        <stop offset="1" stop-color="var(--board-surface-deep)" />
      </linearGradient>
    </defs>

    <rect
      x="0"
      y="0"
      :width="BOARD_WIDTH"
      :height="BOARD_HEIGHT"
      :rx="BOARD_GEOMETRY.boardRadius"
      fill="url(#board-wood)"
    />
    <rect
      :x="BOARD_GEOMETRY.boardInset"
      :y="BOARD_GEOMETRY.boardInset"
      :width="BOARD_WIDTH - BOARD_GEOMETRY.boardInset * 2"
      :height="BOARD_HEIGHT - BOARD_GEOMETRY.boardInset * 2"
      :rx="BOARD_GEOMETRY.innerFrameRadius"
      fill="none"
      stroke="rgba(255,255,255,0.26)"
      :stroke-width="BOARD_GEOMETRY.gridStroke"
    />

    <rect
      :x="VIEW_PIXELS[0][0].x"
      :y="VIEW_PIXELS[0][0].y"
      :width="VIEW_PIXELS[0][8].x - VIEW_PIXELS[0][0].x"
      :height="VIEW_PIXELS[9][0].y - VIEW_PIXELS[0][0].y"
      fill="none"
      stroke="var(--board-frame)"
      :stroke-width="BOARD_GEOMETRY.frameStroke"
    />

    <!-- 单路径合并渲染的全量静态棋盘网格、九宫斜线与兵炮标记（零虚拟 DOM 开销） -->
    <path
      :d="STATIC_GRID_PATH"
      fill="none"
      stroke="var(--board-line)"
      :stroke-width="BOARD_GEOMETRY.gridStroke"
      stroke-linecap="round"
      stroke-linejoin="round"
    />

    <g v-if="preferences.showCoordinates" fill="var(--board-line)" :font-size="BOARD_GEOMETRY.coordinateFontSize" font-weight="600">
      <text v-for="(item, c) in coordinateLabels.top" :key="`top-fc${c}`" :x="item.x" :y="VIEW_PIXELS[0][0].y - BOARD_GEOMETRY.coordinateOffset" text-anchor="middle">
        {{ item.label }}
      </text>
      <text v-for="(item, c) in coordinateLabels.bottom" :key="`bottom-fc${c}`" :x="item.x" :y="BOARD_HEIGHT - BOARD_GEOMETRY.coordinateOffset" text-anchor="middle">
        {{ item.label }}
      </text>
    </g>

    <text
      :x="VIEW_PIXELS[4][2].x"
      :y="riverCenterY"
      text-anchor="middle"
      dominant-baseline="central"
      fill="var(--board-river)"
      :font-size="BOARD_GEOMETRY.riverFontSize"
      font-weight="700"
      :letter-spacing="BOARD_GEOMETRY.riverLetterSpacing"
    >
      楚河
    </text>
    <text
      :x="VIEW_PIXELS[4][6].x"
      :y="riverCenterY"
      text-anchor="middle"
      dominant-baseline="central"
      fill="var(--board-river)"
      :font-size="BOARD_GEOMETRY.riverFontSize"
      font-weight="700"
      :letter-spacing="BOARD_GEOMETRY.riverLetterSpacing"
    >
      汉界
    </text>

    <defs>
      <filter id="engine-arrow-shadow" x="-30%" y="-30%" width="160%" height="160%">
        <feDropShadow dx="0" dy="1.5" stdDeviation="2" flood-color="#000000" flood-opacity="0.38" />
      </filter>
    </defs>

    <!-- 键盘焦点指示标记（双层高对比度描边，底圈黑边+顶圈亮金，在任何木质或深浅色底板下均具备极佳对比度） -->
    <g v-if="shouldShowFocusMark()">
      <path
        v-for="(path, index) in makeCornerPaths(point((focused ?? activeSquare)[0], (focused ?? activeSquare)[1]))"
        :key="`focus-corner-bg-${index}`"
        :d="path"
        fill="none"
        stroke="rgba(0, 0, 0, 0.75)"
        :stroke-width="BOARD_GEOMETRY.gridStroke * 2.4"
        stroke-linecap="round"
      />
      <path
        v-for="(path, index) in makeCornerPaths(point((focused ?? activeSquare)[0], (focused ?? activeSquare)[1]))"
        :key="`focus-corner-${index}`"
        :d="path"
        fill="none"
        stroke="var(--board-focus)"
        :stroke-width="BOARD_GEOMETRY.gridStroke * 1.4"
        stroke-linecap="round"
      />
    </g>

    <!-- 最近一步走棋标记（目标落子格四角 + 起点格圆环点） -->
    <template v-if="!props.editorMode && renderedLastMove">
      <path
        v-for="(path, index) in makeCornerPaths(
          point(renderedLastMove.to[0], renderedLastMove.to[1])
        )"
        :key="`last-move-to-${index}`"
        :d="path"
        fill="none"
        stroke="var(--board-last-move)"
        :stroke-width="BOARD_GEOMETRY.checkStroke"
        stroke-linecap="round"
        opacity="0.75"
      />
      <g :transform="boardTransform(renderedLastMove.from, preferences.boardOrientation, renderedTurn)" opacity="0.75">
        <circle
          :r="BOARD_GEOMETRY.selectionOuter * 0.42"
          fill="none"
          stroke="var(--board-last-move)"
          :stroke-width="BOARD_GEOMETRY.checkStroke"
        />
        <circle :r="BOARD_GEOMETRY.hintRadius" fill="var(--board-last-move)" />
      </g>
    </template>

    <g v-if="renderedSelection" fill="none" stroke="var(--board-selection-edge)" :stroke-width="BOARD_GEOMETRY.frameStroke * 0.875" stroke-linecap="round">
      <path
        v-for="(path, index) in makeCornerPaths(point(renderedSelection[0], renderedSelection[1]))"
        :key="`selected-corner-${index}`"
        :d="path"
        stroke-linejoin="round"
      />
    </g>

    <!-- 静态棋子渲染（目标格在动画期间置空，复活棋子立即呈现） -->
    <template v-for="(row, r) in visualBoard" :key="`p${r}`">
      <template v-for="(p, c) in row" :key="`p${r}-${c}`">
        <g v-if="p" :transform="boardTransform([r, c], preferences.boardOrientation, renderedTurn)">
          <circle
            v-if="!props.editorMode && renderedInCheck && pieceColor(p) === renderedTurn && isKing(p)"
            :r="BOARD_GEOMETRY.checkRadius"
            fill="none"
            stroke="var(--board-check)"
            :stroke-width="BOARD_GEOMETRY.checkStroke"
          />
          <circle :r="BOARD_GEOMETRY.pieceRadius" :fill="pieceColor(p) === 'red' ? 'var(--board-red)' : 'var(--board-black)'" stroke="var(--board-line)" :stroke-width="BOARD_GEOMETRY.gridStroke" />
          <circle :r="BOARD_GEOMETRY.pieceFaceRadius" fill="var(--piece-face)" />
          <text
            text-anchor="middle"
            dominant-baseline="central"
            :fill="pieceColor(p) === 'red' ? 'var(--board-red)' : 'var(--board-black)'"
            :font-size="BOARD_GEOMETRY.pieceFontSize"
            font-weight="700"
          >
            {{ pieceGlyph(p) }}
          </text>
        </g>
      </template>
    </template>

    <!-- 屏幕物理刷新率驱动的直通 DOM 落子动画层（零响应式延迟，零提前截断） -->
    <g
      v-for="m in activeMotions"
      :key="m.id"
      :ref="(el) => setMotionRef(m.id, el as SVGElement | null)"
      :transform="`translate(${m.fromX}, ${m.fromY})`"
      pointer-events="none"
      aria-hidden="true"
    >
      <circle :r="BOARD_GEOMETRY.pieceRadius" :fill="pieceColor(m.piece) === 'red' ? 'var(--board-red)' : 'var(--board-black)'" stroke="var(--board-line)" :stroke-width="BOARD_GEOMETRY.gridStroke" />
      <circle :r="BOARD_GEOMETRY.pieceFaceRadius" fill="var(--piece-face)" />
      <text
        text-anchor="middle"
        dominant-baseline="central"
        :fill="pieceColor(m.piece) === 'red' ? 'var(--board-red)' : 'var(--board-black)'"
        :font-size="BOARD_GEOMETRY.pieceFontSize"
        font-weight="700"
      >
        {{ pieceGlyph(m.piece) }}
      </text>
    </g>

    <!-- 引擎箭头在棋子与动画层之后绘制，确保提示始终显示在棋子上方 -->
    <g pointer-events="none" filter="url(#engine-arrow-shadow)">
      <path
        v-for="arrow in engineArrows"
        :key="`arrow-${arrow.id}`"
        :d="arrow.path"
        :fill="arrow.color"
        :fill-opacity="arrow.opacity"
        stroke="var(--background)"
        stroke-width="0.75"
        :stroke-opacity="arrow.opacity * 0.5"
        stroke-linejoin="round"
      />
    </g>

    <template v-if="!props.editorMode" v-for="t in legalTargets" :key="`t${t[0]}-${t[1]}`">
      <circle
        :cx="point(t[0], t[1]).x"
        :cy="point(t[0], t[1]).y"
        :r="BOARD_GEOMETRY.hintRadius"
        fill="var(--board-hint)"
        opacity="0.75"
      />
    </template>

    <!-- 90 个交互点击检测层（使用预计算坐标表，Roving Tabindex 单一活动焦点） -->
    <circle
      v-for="s in hitTargets"
      :key="`c${s.row}-${s.col}`"
      :ref="(el) => setSquareRef(s.row, s.col, el)"
      class="board-hit-target"
      :cx="s.x"
      :cy="s.y"
      :r="BOARD_GEOMETRY.hitRadius"
      fill="transparent"
      role="button"
      :aria-label="squareLabels[s.row * 9 + s.col]"
      :aria-pressed="renderedSelection && renderedSelection[0] === s.row && renderedSelection[1] === s.col ? true : undefined"
      :tabindex="activeSquare[0] === s.row && activeSquare[1] === s.col ? 0 : -1"
      @click="onBoardSquareClick(s.row, s.col)"
      @contextmenu.prevent="onBoardSquareRightClick(s.row, s.col)"
      @pointerdown="onBoardSquarePointerDown"
      @focus="onBoardSquareFocus(s.row, s.col)"
      @blur="onBoardSquareBlur"
      @keydown="onSquareKeydown($event, s.row, s.col)"
    />
  </svg>
</template>
