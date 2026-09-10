<script setup lang="ts">
import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  Pause,
  Play,
} from "@lucide/vue";
import { Button } from "@/components/ui";
import { useGameStore } from "@/stores/game";
import { useReplay } from "@/composables/useReplay";

const game = useGameStore();
const { isPlaying: playing, togglePlayback, stopPlayback } = useReplay();

async function first() {
  stopPlayback();
  await game.jumpTo(0);
}

async function previous() {
  stopPlayback();
  if (game.canUndo) await game.undo();
}

async function next() {
  stopPlayback();
  if (game.canRedo) await game.redo();
}

async function last() {
  stopPlayback();
  await game.jumpTo(game.history.length);
}
</script>

<template>
  <div class="flex items-center justify-between gap-1 border-t bg-card/60 px-3 py-2 text-xs">
    <span class="truncate text-body-sm text-muted-foreground">
      {{ game.currentPly === 0 ? "起始局面" : `第 ${game.currentPly} / ${game.history.length} 步` }}
    </span>
    <div class="flex items-center gap-0.5" aria-label="复盘导航">
      <Button variant="ghost" size="icon" class="size-6" :disabled="!game.canUndo" aria-label="起始局面 (Home)" title="起始局面 (Home / Shift+[)" @click="first">
        <ChevronsLeft class="size-3.5" />
      </Button>
      <Button variant="ghost" size="icon" class="size-6" :disabled="!game.canUndo" aria-label="上一步 ([)" title="上一步 ([ / Ctrl+Z)" @click="previous">
        <ChevronLeft class="size-3.5" />
      </Button>
      <Button variant="secondary" size="icon" class="size-6" :disabled="!game.canRedo" :aria-label="playing ? '暂停播放 (Space)' : '自动播放 (Space)'" :title="playing ? '暂停播放 (Space)' : '自动播放 (Space)'" @click="togglePlayback">
        <Pause v-if="playing" class="size-3.5" />
        <Play v-else class="size-3.5" />
      </Button>
      <Button variant="ghost" size="icon" class="size-6" :disabled="!game.canRedo" aria-label="下一步 (])" title="下一步 (] / Ctrl+Y)" @click="next">
        <ChevronRight class="size-3.5" />
      </Button>
      <Button variant="ghost" size="icon" class="size-6" :disabled="!game.canRedo" aria-label="最新局面 (End)" title="最新局面 (End / Shift+])" @click="last">
        <ChevronsRight class="size-3.5" />
      </Button>
    </div>
  </div>
</template>
