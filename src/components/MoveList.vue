<script setup lang="ts">
import { computed, ref } from "vue";
import { Badge, Button, ScrollArea } from "@/components/ui";
import MoveAnnotationEditor from "@/components/MoveAnnotationEditor.vue";
import { useGameStore } from "@/stores/game";
import { useManualStore } from "@/stores/manual";
import type { UnifiedPly } from "@/stores/game";

const game = useGameStore();
const manual = useManualStore();
const navigating = ref(false);

interface MoveRound {
  round: number;
  red?: UnifiedPly;
  black?: UnifiedPly;
}

const rounds = computed<MoveRound[]>(() => {
  const result: MoveRound[] = [];
  const list = game.history;
  for (let i = 0; i < list.length; i += 2) {
    result.push({
      round: Math.floor(i / 2) + 1,
      red: list[i],
      black: list[i + 1],
    });
  }
  return result;
});

async function jumpTo(ply: number) {
  if (navigating.value) return;
  navigating.value = true;
  try {
    await game.jumpTo(ply);
    manual.selectPly(ply);
  } finally {
    navigating.value = false;
  }
}

function hasAnnotation(ply: number) {
  return !!manual.annotationForPly(ply);
}

async function openBranchNode(nodeId: number) {
  if (navigating.value) return;
  const path = manual.pathToNode(nodeId);
  if (!path) return;
  navigating.value = true;
  try {
    await manual.applyNodes(path);
  } finally {
    navigating.value = false;
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col bg-card">
    <header class="flex min-h-9 shrink-0 flex-wrap items-center justify-between gap-1 border-b px-2.5 py-1 sm:px-3">
      <div class="flex min-w-0 items-center gap-1.5">
        <h3 class="text-xs font-semibold text-foreground">着法记录</h3>
        <Badge variant="secondary" class="h-4 px-1 text-caption font-normal">
          {{ game.history.length }} 手
        </Badge>
      </div>
      <div class="ml-auto flex shrink-0 items-center gap-1">
        <Button
          variant="ghost"
          size="sm"
          class="h-6 px-2 text-body-sm"
          title="悔棋 ([ / Ctrl+Z)"
          :disabled="navigating || !game.canUndo"
          @click="game.undo"
        >
          悔棋
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-6 px-2 text-body-sm"
          title="重做 (] / Ctrl+Y)"
          :disabled="navigating || !game.canRedo"
          @click="game.redo"
        >
          重做
        </Button>
      </div>
    </header>

    <ScrollArea class="flex-1 min-h-0 px-2 py-1.5">
      <div class="flex flex-col gap-1 pr-1">
        <!-- 起始局面按钮 -->
        <button
          type="button"
          class="flex h-8 w-full items-center justify-center rounded-md border text-xs transition-colors hover:bg-accent"
          :class="
            game.currentPly === 0
              ? 'border-primary bg-accent font-semibold text-accent-foreground ring-1 ring-primary'
              : 'border-transparent text-muted-foreground'
          "
          :disabled="navigating"
          @click="jumpTo(0)"
        >
          — 开局 —
        </button>

        <!-- 双列排版：回合 · 红方 · 黑方 -->
        <div
          v-for="item in rounds"
          :key="item.round"
          class="grid grid-cols-[1.5rem_1fr_1fr] items-center gap-1.5 py-0.5"
        >
          <!-- 回合标号 -->
          <span class="text-right text-body-sm font-mono text-muted-foreground select-none pr-0.5">
            {{ item.round }}.
          </span>

          <!-- 红方着法 -->
          <button
            v-if="item.red"
            type="button"
            class="flex h-8 items-center justify-center rounded-sm px-1.5 text-xs whitespace-nowrap transition-colors hover:bg-accent focus-visible:outline-none"
            :class="[
              game.currentPly === item.red.ply
                ? 'bg-primary/15 font-semibold text-side-red-fg ring-1 ring-primary'
                : item.red.ply <= game.currentPly
                  ? 'text-side-red-fg'
                  : 'text-side-red-fg/40',
            ]"
            :disabled="navigating"
            @click="jumpTo(item.red.ply)"
          >
            <span>{{ item.red.notation }}</span>
            <span v-if="item.red.public_detail" class="ml-1 text-caption text-muted-foreground">{{ item.red.public_detail }}</span>
            <span v-if="hasAnnotation(item.red.ply)" class="ml-1 text-primary" title="有备注" aria-label="有备注">●</span>
            <span v-if="item.red.is_check" class="ml-0.5 text-caption font-bold text-destructive">+</span>
          </button>
          <span v-else />

          <!-- 黑方着法 -->
          <button
            v-if="item.black"
            type="button"
            class="flex h-8 items-center justify-center rounded-sm px-1.5 text-xs whitespace-nowrap transition-colors hover:bg-accent focus-visible:outline-none"
            :class="[
              game.currentPly === item.black.ply
                ? 'bg-accent font-semibold text-foreground ring-1 ring-primary'
                : item.black.ply <= game.currentPly
                  ? 'text-foreground'
                  : 'text-muted-foreground/50',
            ]"
            :disabled="navigating"
            @click="jumpTo(item.black.ply)"
          >
            <span>{{ item.black.notation }}</span>
            <span v-if="item.black.public_detail" class="ml-1 text-caption text-muted-foreground">{{ item.black.public_detail }}</span>
            <span v-if="hasAnnotation(item.black.ply)" class="ml-1 text-primary" title="有备注" aria-label="有备注">●</span>
            <span v-if="item.black.is_check" class="ml-0.5 text-caption font-bold text-destructive">+</span>
          </button>
          <span v-else />
        </div>

        <div v-if="game.history.length === 0" class="py-8 text-center text-xs text-muted-foreground">
          尚无走子记录
        </div>

        <!-- Lichess 风格变例分支提示块 -->
        <div v-if="manual.nextBranches.length > 1" class="mt-2 rounded-md border border-dashed border-border bg-muted/30 p-2">
          <p class="mb-1.5 text-body-sm font-medium text-muted-foreground">变例分支 (Variations)</p>
          <div class="flex flex-col gap-1">
            <button
              v-for="(node, branchIndex) in manual.nextBranches"
              :key="node.id"
              type="button"
              class="flex h-6 w-full items-center justify-between rounded px-2 text-xs transition-colors hover:bg-accent"
              :disabled="navigating"
              @click="openBranchNode(node.id)"
            >
              <span class="text-body-sm text-muted-foreground">
                {{ branchIndex === 0 ? "主线" : `变例 ${branchIndex}` }}
              </span>
              <span class="flex items-center gap-1 font-medium text-foreground">
                {{ node.chinese_notation }}
                <span v-if="node.comment" class="text-primary" title="有备注" aria-label="有备注">●</span>
              </span>
            </button>
          </div>
        </div>

        <MoveAnnotationEditor v-if="game.capabilities.edit_annotations.enabled" class="mt-2" />
      </div>
    </ScrollArea>
  </div>
</template>
