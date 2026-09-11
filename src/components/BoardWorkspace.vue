<script setup lang="ts">
import { computed, ref } from "vue";
import { Badge, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui";
import { resultLabel, statusLabel } from "@/lib/presentation";
import { useGameStore } from "@/stores/game";
import { useEngineStore } from "@/stores/engine";
import { iccsToCoords, parseFen, type BoardViewModel } from "@/lib/chess";
import ChessBoard from "./ChessBoard.vue";
import BoardControls from "./BoardControls.vue";

const game = useGameStore();
const engine = useEngineStore();
const chessBoardRef = ref<{ selected: string | null } | null>(null);

const previewView = computed<BoardViewModel | null>(() => {
  const snapshot = engine.previewSnapshot;
  if (!snapshot?.fen) return null;
  const { board, turn } = parseFen(snapshot.fen);
  const lastMove = snapshot.last_move_iccs ? iccsToCoords(snapshot.last_move_iccs) : null;
  return {
    board,
    turn,
    inCheck: snapshot.in_check,
    lastMove,
  };
});

function openPosition() {
  emit("open-position");
}

const emit = defineEmits<{
  (event: "open-position"): void;
}>();

defineExpose({
  selected: computed(() => chessBoardRef.value?.selected ?? null),
});
</script>

<template>
  <section class="board-workspace flex h-full min-h-0 min-w-0 flex-col overflow-hidden rounded-xl border bg-card shadow-xs">
    <header class="flex h-9 shrink-0 items-center justify-between gap-3 border-b px-4">
      <div class="flex min-w-0 items-center gap-2.5">
        <span
          class="turn-indicator"
          :class="game.redToMove ? 'turn-indicator--red' : 'turn-indicator--black'"
          aria-hidden="true"
        />
        <div class="flex min-w-0 items-center gap-2">
          <span class="truncate text-xs font-semibold text-foreground">
            {{ game.result === "ongoing" ? (game.redToMove ? "红方走棋" : "黑方走棋") : resultLabel(game.result) }}
          </span>
          <span class="shrink-0 text-body-sm text-muted-foreground">
            第 {{ game.appliedHistory.length + 1 }} 手
          </span>
        </div>
      </div>

      <div class="flex min-w-0 items-center justify-end gap-2">
        <Badge
          v-if="game.inCheck && game.result === 'ongoing'"
          variant="destructive"
          class="h-5 shrink-0 px-1.5 text-caption font-semibold animate-pulse"
        >
          将军!
        </Badge>
        <Tooltip v-if="game.ruleStatus !== 'ongoing' && game.ruleExplanation">
          <TooltipTrigger as-child>
            <Badge
              variant="outline"
              class="h-5 max-w-[12rem] shrink-0 cursor-pointer truncate border-amber-500/60 px-1.5 text-caption text-amber-600 dark:text-amber-400"
            >
              规则参考: {{ statusLabel(game.ruleStatus, game.result) }}
            </Badge>
          </TooltipTrigger>
          <TooltipContent class="max-w-xs text-xs">
            {{ game.ruleExplanation.message }}
          </TooltipContent>
        </Tooltip>
        <span class="hidden shrink-0 text-body-sm text-muted-foreground sm:inline">
          {{ game.variant === "jieqi" ? (game.playMode === "duel" ? "揭棋 · 对弈" : "揭棋 · 训练") : game.ruleProfile === "china2020" ? "中国规则 2020" : "亚洲/世界规则 2017" }}
        </span>
      </div>
    </header>

    <div class="flex min-h-0 flex-1 items-center justify-center overflow-hidden p-2 sm:p-3">
      <ChessBoard
        ref="chessBoardRef"
        class="h-full w-full"
        :view="previewView"
        :previewing="Boolean(engine.previewSnapshot)"
      />
    </div>

    <BoardControls class="w-full" :open-position="openPosition" />
  </section>
</template>
