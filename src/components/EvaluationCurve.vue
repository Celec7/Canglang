<script setup lang="ts">
import { computed, ref } from "vue";
import { useGameStore } from "@/stores/game";
import { useEngineStore } from "@/stores/engine";
import { formatEvaluation } from "@/lib/engine-evaluation";

const game = useGameStore();
const engine = useEngineStore();

const width = 300;
const height = 80;
const padTop = 10;
const padBottom = 16;
const padX = 14;

const hoveredPly = ref<number | null>(null);

interface PointData {
  ply: number;
  notation: string;
  mover: "red" | "black";
  winRate: number;
  score: number;
  mateIn: number | null;
  hasEval: boolean;
  x: number;
  y: number;
}

const chartPoints = computed<PointData[]>(() => {
  const history = game.history;
  const totalPlies = history.length;
  const evals = engine.gameEvaluations;

  const plotW = width - padX * 2;
  const plotH = height - padTop - padBottom;

  const points: PointData[] = [];

  // ply 0 (初始局，positionKey 为空字符串)
  const p0Eval = evals[0]?.positionKey === "" ? evals[0] : undefined;
  const p0WinRate = p0Eval?.winRate ?? 0.5;
  points.push({
    ply: 0,
    notation: "开局",
    mover: "red",
    winRate: p0WinRate,
    score: p0Eval?.score ?? 0,
    mateIn: p0Eval?.mateIn ?? null,
    hasEval: Boolean(p0Eval),
    x: padX,
    y: padTop + (1 - p0WinRate) * plotH,
  });

  let lastKnownWinRate = p0WinRate;
  let currentKey = "";

  for (let i = 0; i < totalPlies; i++) {
    const rec = history[i];
    const ply = i + 1;
    currentKey = currentKey ? `${currentKey} ${rec.iccs}` : rec.iccs;
    const evaluation = evals[ply];
    const ev = evaluation?.positionKey === currentKey ? evaluation : undefined;
    const winRate = ev ? ev.winRate : lastKnownWinRate;
    if (ev) lastKnownWinRate = ev.winRate;

    const x = padX + (ply / Math.max(1, totalPlies)) * plotW;
    const y = padTop + (1 - winRate) * plotH;

    points.push({
      ply,
      notation: rec.notation || rec.iccs,
      mover: rec.mover === "red" ? "red" : "black",
      winRate,
      score: ev?.score ?? 0,
      mateIn: ev?.mateIn ?? null,
      hasEval: Boolean(ev),
      x,
      y,
    });
  }

  return points;
});

const polylinePoints = computed(() => {
  if (chartPoints.value.length < 2) return "";
  return chartPoints.value.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" ");
});

const currentPoint = computed(() => {
  const cur = game.currentPly;
  return chartPoints.value.find((p) => p.ply === cur) ?? chartPoints.value[chartPoints.value.length - 1];
});

const activeHover = computed(() => {
  if (hoveredPly.value === null) return null;
  return chartPoints.value.find((p) => p.ply === hoveredPly.value) ?? null;
});

async function onSelectPly(ply: number) {
  await game.jumpTo(ply);
}
</script>

<template>
  <div class="rounded-lg border border-border/70 bg-card/60 p-2.5 min-w-0">
    <!-- 标题与状态提示 -->
    <div class="mb-1 flex items-center justify-between text-xs text-muted-foreground min-w-0">
      <div class="flex items-center gap-1.5 font-medium text-foreground">
        <span>整局胜率走势</span>
        <span class="text-[10px] font-normal text-muted-foreground">
          ({{ game.history.length }} 手)
        </span>
      </div>

      <div class="flex items-center gap-2 text-[11px]">
        <span class="text-side-red-fg font-semibold">红优 ▲</span>
        <span class="text-muted-foreground">|</span>
        <span class="text-foreground font-semibold">黑优 ▼</span>
      </div>
    </div>

    <!-- 交互式 SVG 图表 -->
    <div class="relative w-full overflow-hidden select-none">
      <svg
        :viewBox="`0 0 ${width} ${height}`"
        class="h-20 w-full overflow-visible"
        role="img"
        aria-label="整局胜率走势图"
      >
        <!-- 50% 中轴均势参考虚线 -->
        <line
          :x1="padX"
          :x2="width - padX"
          :y1="padTop + (height - padTop - padBottom) / 2"
          :y2="padTop + (height - padTop - padBottom) / 2"
          stroke="currentColor"
          stroke-opacity="0.22"
          stroke-dasharray="3 3"
        />

        <!-- 折线走势 -->
        <polyline
          v-if="polylinePoints"
          :points="polylinePoints"
          fill="none"
          stroke="hsl(var(--primary))"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />

        <!-- 当前手序指示线与圆点 -->
        <g v-if="currentPoint">
          <line
            :x1="currentPoint.x"
            :x2="currentPoint.x"
            :y1="padTop"
            :y2="height - padBottom"
            stroke="hsl(var(--primary))"
            stroke-width="1.5"
            stroke-dasharray="2 2"
            opacity="0.8"
          />
          <circle
            :cx="currentPoint.x"
            :cy="currentPoint.y"
            r="3.5"
            fill="hsl(var(--primary))"
            stroke="hsl(var(--background))"
            stroke-width="1.5"
          />
        </g>

        <!-- 悬停指示 -->
        <g v-if="activeHover && activeHover.ply !== currentPoint?.ply">
          <circle
            :cx="activeHover.x"
            :cy="activeHover.y"
            r="3"
            fill="currentColor"
            opacity="0.75"
          />
        </g>

        <!-- 可点击的热区列 -->
        <rect
          v-for="p in chartPoints"
          :key="`hit-${p.ply}`"
          :x="p.x - Math.max(4, (width - padX * 2) / (chartPoints.length * 2))"
          :y="padTop"
          :width="Math.max(8, (width - padX * 2) / chartPoints.length)"
          :height="height - padTop - padBottom"
          fill="transparent"
          class="cursor-pointer"
          @mouseenter="hoveredPly = p.ply"
          @mouseleave="hoveredPly = null"
          @click="onSelectPly(p.ply)"
        />
      </svg>

      <!-- 悬停或当前步详细信息悬浮气泡 -->
      <div
        v-if="activeHover || currentPoint"
        class="mt-1 flex items-center justify-between text-[11px] text-muted-foreground border-t pt-1"
      >
        <span class="truncate">
          第 {{ (activeHover ?? currentPoint)!.ply }} 手:
          <strong class="text-foreground ml-1">
            {{ (activeHover ?? currentPoint)!.notation }}
          </strong>
        </span>
        <span class="font-mono tabular-nums">
          红方胜率:
          <strong
            :class="(activeHover ?? currentPoint)!.winRate >= 0.5 ? 'text-side-red-fg' : 'text-foreground'"
          >
            {{ Math.round((activeHover ?? currentPoint)!.winRate * 100) }}%
          </strong>
          <span v-if="(activeHover ?? currentPoint)!.hasEval" class="ml-1 text-[10px]">
            ({{ formatEvaluation((activeHover ?? currentPoint)!.score, (activeHover ?? currentPoint)!.mateIn) }})
          </span>
        </span>
      </div>
    </div>
  </div>
</template>
