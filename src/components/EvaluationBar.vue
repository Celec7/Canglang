<script setup lang="ts">
import { computed } from "vue";
import { formatEvaluation, scoreToWinRate } from "@/lib/engine-evaluation";

const props = defineProps<{ score: number | null; mateIn?: number | null }>();
const winRate = computed(() => (props.score === null ? 0.5 : scoreToWinRate(props.score)));
const label = computed(() => (props.score === null ? "—" : formatEvaluation(props.score, props.mateIn ?? null)));
</script>

<template>
  <div class="flex items-center gap-2.5 min-w-0" aria-label="当前局面评估">
    <!-- 评分/绝杀标签 -->
    <span
      class="min-w-14 text-left font-mono text-xs font-bold tabular-nums shrink-0"
      :class="[
        props.mateIn ? 'text-destructive animate-pulse' : (props.score ?? 0) >= 0 ? 'text-side-red-fg' : 'text-foreground'
      ]"
    >
      {{ label }}
    </span>

    <!-- 胜率横条 (红方胜率宽度) -->
    <div
      class="relative h-2 min-w-0 flex-1 overflow-hidden rounded-full bg-slate-800/80 shadow-inner"
      role="meter"
      :aria-valuenow="Math.round(winRate * 100)"
      aria-valuemin="0"
      aria-valuemax="100"
    >
      <div
        class="h-full rounded-full bg-side-red-fg/90 transition-[width] duration-300"
        :style="{ width: `${winRate * 100}%` }"
      />
      <span
        class="absolute inset-y-0 w-px bg-white/80"
        :style="{ left: `${winRate * 100}%` }"
        aria-hidden="true"
      />
    </div>

    <!-- 胜率百分比数值 -->
    <span class="w-10 text-right text-xs font-mono text-muted-foreground tabular-nums shrink-0">
      {{ Math.round(winRate * 100) }}%
    </span>
  </div>
</template>
