<script setup lang="ts">
import { computed, ref } from "vue";
import { Check, Copy } from "@lucide/vue";
import type { ThinkData } from "@/stores/engine";
import {
  formatElapsed,
  formatNps,
  formatScoreDetailed,
  toRedPerspective,
} from "@/lib/engine-evaluation";
import { useGameStore } from "@/stores/game";
import { useToast } from "@/composables/useToast";

const game = useGameStore();

const props = defineProps<{
  list?: ThinkData[];
  think?: ThinkData | null;
  winRate?: number | null;
}>();
const emit = defineEmits<{ preview: [moves: string[]] }>();

const { show } = useToast();
const copiedIndex = ref<number | null>(null);

const displayList = computed<ThinkData[]>(() => {
  if (props.list && props.list.length > 0) {
    return props.list;
  }
  return props.think ? [props.think] : [];
});

const rankColors = ["#2563eb", "#0284c7", "#0d9488", "#64748b", "#94a3b8"];

function itemDetailed(item: ThinkData) {
  const { score, mateIn } = toRedPerspective(item.score, item.mate_in, game.redToMove);
  return formatScoreDetailed(score, mateIn);
}

function getMoves(item: ThinkData): string[] {
  return item.pv_chinese && item.pv_chinese.length > 0 ? item.pv_chinese : item.pv;
}

function getPvText(item: ThinkData): string {
  return getMoves(item).join("  ");
}

function deltaScoreText(item: ThinkData, idx: number): string | null {
  if (idx === 0 || displayList.value.length < 2) return null;
  const best = displayList.value[0];
  if (item.mate_in !== null || best.mate_in !== null) return null;
  const diff = (item.score - best.score) / 100;
  return `(${diff >= 0 ? "+" : ""}${diff.toFixed(2)})`;
}

async function copyPv(item: ThinkData, index: number) {
  const text = getPvText(item);
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    copiedIndex.value = index;
    show("已复制变例招法到剪贴板");
    setTimeout(() => {
      if (copiedIndex.value === index) {
        copiedIndex.value = null;
      }
    }, 1800);
  } catch {
    show("复制失败");
  }
}
</script>

<template>
  <div class="flex flex-col gap-2.5 min-w-0">
    <template v-if="displayList.length > 0">
      <div
        v-for="(item, idx) in displayList"
        :key="item.multi_pv || idx"
        class="flex flex-col gap-1.5 rounded-lg border bg-card/70 p-2.5 text-xs min-w-0 transition-colors"
        :class="idx === 0 ? 'border-primary/50 shadow-xs' : 'border-border/60'"
      >
        <!-- 单行紧凑等宽指标头: 深度 / 红分 (或绝杀) / 耗时 / NPS + 复制按钮 -->
        <div class="flex items-center justify-between gap-1.5 border-b border-border/40 pb-1.5 min-w-0 text-muted-foreground font-mono text-[11px] tabular-nums">
          <div class="flex flex-wrap items-center gap-x-2.5 gap-y-1 min-w-0">
            <!-- 候选序数 -->
            <span
              class="flex size-4 items-center justify-center rounded-full text-[9px] font-bold text-white shrink-0"
              :style="{ backgroundColor: rankColors[idx] ?? '#64748b' }"
              :title="`候选分支 ${item.multi_pv || idx + 1}`"
            >
              {{ item.multi_pv || idx + 1 }}
            </span>

            <!-- 深度 -->
            <span>
              深度: <strong class="text-foreground">{{ item.depth }}</strong>
            </span>

            <!-- 红分 / 绝杀状态 -->
            <span
              :class="[
                itemDetailed(item).isMate
                  ? 'font-bold text-destructive animate-pulse'
                  : itemDetailed(item).isRed
                  ? 'font-medium text-side-red-fg'
                  : 'font-medium text-foreground'
              ]"
            >
              {{ itemDetailed(item).label }}
              <span v-if="deltaScoreText(item, idx)" class="text-[10px] text-muted-foreground font-normal ml-0.5">
                {{ deltaScoreText(item, idx) }}
              </span>
            </span>

            <!-- 耗时 -->
            <span>
              耗时: <span class="text-foreground">{{ formatElapsed(item.time_ms) }}</span>
            </span>

            <!-- NPS 算力吞吐 -->
            <span>
              NPS: <span class="text-foreground">{{ formatNps(item.nps) }}</span>
            </span>
          </div>

          <!-- 一键复制变例 -->
          <button
            type="button"
            class="flex size-6 shrink-0 items-center justify-center rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
            title="复制本条推演招法"
            @click="copyPv(item, idx)"
          >
            <Check v-if="copiedIndex === idx" class="size-3.5 text-emerald-500" />
            <Copy v-else class="size-3.5" />
          </button>
        </div>

        <!-- 中文推演长招法流序列 -->
        <div
          v-if="getMoves(item).length > 0"
          class="font-normal text-foreground leading-relaxed text-[11px] tracking-wide break-words select-text pt-0.5"
        >
          <template v-for="(mv, mIdx) in getMoves(item)" :key="mIdx">
            <button type="button" class="inline-block whitespace-nowrap hover:text-primary hover:font-medium transition-colors cursor-pointer" @click="emit('preview', item.pv.slice(0, mIdx + 1))">
              {{ mv }}
            </button>
            <span v-if="mIdx < getMoves(item).length - 1" class="text-muted-foreground/40 mx-1 select-none">
              ·
            </span>
          </template>
        </div>
      </div>
    </template>

    <div v-else class="rounded-lg border border-dashed p-4 text-center text-xs text-muted-foreground">
      等待引擎实时分析推演...
    </div>
  </div>
</template>
