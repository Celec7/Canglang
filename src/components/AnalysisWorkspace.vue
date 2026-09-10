<script setup lang="ts">
import { computed } from "vue";
import { Cpu, Shuffle, Zap } from "@lucide/vue";
import { Badge, Button, ScrollArea, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui";
import { useEngineStore } from "@/stores/engine";
import { useGameStore } from "@/stores/game";
import { scoreToWinRate, toRedPerspective } from "@/lib/engine-evaluation";
import { useToast } from "@/composables/useToast";
import EvaluationBar from "./EvaluationBar.vue";
import EvaluationCurve from "./EvaluationCurve.vue";
import MultiPvList from "./MultiPvList.vue";
import BookTable from "./BookTable.vue";
import AnalysisPreviewBar from "./AnalysisPreviewBar.vue";
import EngineLogPanel from "./EngineLogPanel.vue";

const engine = useEngineStore();
const game = useGameStore();
const { show } = useToast();

const currentEvaluation = computed(() => {
  const td = engine.latestThink;
  if (td) {
    const res = toRedPerspective(td.score, td.mate_in, game.redToMove);
    return { score: res.score, mateIn: res.mateIn };
  }

  if (engine.lastEvaluation) {
    return {
      score: engine.lastEvaluation.score,
      mateIn: engine.lastEvaluation.mateIn,
    };
  }

  return { score: null, mateIn: null };
});

const redPerspective = computed(() => {
  return currentEvaluation.value;
});

const winRate = computed<number | null>(() => {
  if (redPerspective.value.score === null) return null;
  return scoreToWinRate(redPerspective.value.score);
});

async function moveNow() {
  try {
    await engine.moveNow();
  } catch (cause) {
    show(cause instanceof Error ? cause.message : String(cause));
  }
}

async function changeTactic() {
  try {
    if (!game.capabilities.analyze.enabled || !game.fen) {
      show("当前对局不支持象棋引擎分析");
      return;
    }
    const excludedMove = engine.bestMove ?? engine.latestThink?.pv?.[0];
    const excluded = excludedMove ? [excludedMove] : [];
    await engine.changeTactic(
      game.fen,
      game.appliedHistory.map((p) => p.iccs),
      excluded,
      engine.analysisConfig
    );
  } catch (cause) {
    show(cause instanceof Error ? cause.message : String(cause));
  }
}

async function previewPv(moves: string[]) {
  try {
    await engine.previewPv(moves);
  } catch (cause) {
    show(cause instanceof Error ? cause.message : String(cause));
  }
}

</script>

<template>
  <div class="flex h-full min-h-0 flex-col rounded-xl border bg-card shadow-sm overflow-hidden">
    <!-- 头部：标题、状态与就地操作按钮 -->
    <header class="flex h-9 shrink-0 items-center justify-between border-b px-3">
      <div class="flex items-center gap-2">
        <Cpu class="size-4 text-primary" />
        <h3 class="text-xs font-semibold text-foreground">实时分析</h3>
        <Badge
          :variant="engine.analyzing ? 'default' : engine.running ? 'secondary' : 'outline'"
          class="h-4 px-1.5 text-caption"
        >
          {{ engine.analyzing ? "思考中" : engine.running ? "待命" : "未启动" }}
        </Badge>
      </div>

      <div class="flex items-center gap-1">
        <!-- 立即出招 -->
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              variant="ghost"
              size="icon"
              class="size-6"
              :disabled="!engine.running || !engine.analyzing"
              aria-label="立即出招"
              @click="moveNow"
            >
              <Zap class="size-3.5 text-amber-500" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>立即出招 (Move Now)</TooltipContent>
        </Tooltip>

        <!-- 换一变 -->
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              variant="ghost"
              size="icon"
              class="size-6"
              :disabled="!engine.running || engine.analyzing"
              aria-label="换一变"
              @click="changeTactic"
            >
              <Shuffle class="size-3.5 text-primary" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>换一变 (排除当前最佳走法)</TooltipContent>
        </Tooltip>
      </div>
    </header>

    <p v-if="engine.lastError" role="alert" class="border-b border-destructive/20 bg-destructive/5 px-3 py-1.5 text-xs text-destructive">
      {{ engine.lastError }}
    </p>

    <EngineLogPanel class="mx-3 mt-2 shrink-0" />

    <ScrollArea class="flex-1 min-h-0 [&_[data-radix-scroll-area-viewport]>div]:!h-full [&_[data-radix-scroll-area-viewport]>div]:!flex [&_[data-radix-scroll-area-viewport]>div]:!flex-col">
      <div class="flex flex-col gap-3.5 flex-1 min-h-full p-3">
        <!-- 胜率与分值评估条 -->
        <div class="rounded-lg border bg-background/50 p-2.5 shrink-0">
          <EvaluationBar
            :score="redPerspective.score"
            :mate-in="redPerspective.mateIn"
          />
        </div>

        <!-- 整局评估走势图 -->
        <div class="shrink-0">
          <EvaluationCurve />
        </div>

        <!-- 思考深度与 Multi-PV 分支 -->
        <div class="rounded-lg border bg-background/40 p-2.5 shrink-0">
          <AnalysisPreviewBar v-if="engine.previewSnapshot" :snapshot="engine.previewSnapshot" :move-count="engine.previewMoves.length" @apply="engine.applyPreview" @clear="engine.clearPreview" />
          <MultiPvList :list="engine.multiPvList" :think="engine.latestThink" :win-rate="winRate" @preview="previewPv" />
        </div>

        <!-- 弹性开局库区域（充满分析栏剩余垂直空间） -->
        <div class="flex flex-col flex-1 min-h-[160px] border-t pt-2.5">
          <BookTable class="flex-1 min-h-0" />
        </div>
      </div>
    </ScrollArea>
  </div>
</template>
