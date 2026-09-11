<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Button, Textarea } from "@/components/ui";
import { useGameStore } from "@/stores/game";
import { useManualStore } from "@/stores/manual";

const game = useGameStore();
const manual = useManualStore();
const draft = ref("");
const draftPly = ref(0);

const currentText = computed(() => manual.annotationForPly(game.currentPly) ?? "");
const currentLabel = computed(() => {
  if (game.currentPly === 0) return "起始局面";
  return game.history[game.currentPly - 1]?.notation ?? `第 ${game.currentPly} 手`;
});

watch(
  () => [game.variant, game.currentPly, currentText.value] as const,
  ([, ply, text]) => {
    draftPly.value = ply;
    draft.value = text;
  },
  { immediate: true },
);

function ensureCurrentNode() {
  if (game.variant === "jieqi") return game.capabilities.edit_annotations.enabled;
  if (manual.currentNode) return true;
  return manual.selectPly(game.currentPly);
}

function saveComment() {
  if (!ensureCurrentNode()) return;
  if (game.variant === "jieqi") manual.updateJieqiComment(draftPly.value, draft.value);
  else if (manual.currentNode) manual.updateComment(manual.currentNode.id, draft.value);
}

function clearComment() {
  draft.value = "";
  saveComment();
}

function restoreComment() {
  draft.value = currentText.value;
}
</script>

<template>
  <section class="border-l-2 border-primary/70 bg-primary/[0.04] px-2.5 py-2" aria-label="着法备注">
    <template v-if="game.variant === 'jieqi' || manual.currentNode">
      <div class="flex items-center justify-between gap-2">
        <div class="min-w-0">
          <p class="text-body-sm font-semibold text-foreground">着法备注</p>
          <p class="truncate text-caption text-muted-foreground">
            {{ currentLabel }}
          </p>
        </div>
        <span v-if="manual.dirty" class="shrink-0 text-caption text-muted-foreground">未保存</span>
      </div>

      <Textarea
        v-model="draft"
        rows="4"
        class="mt-2 resize-y border-primary/30 bg-background text-body-sm leading-relaxed"
        placeholder="记录这个局面的思路、变化或复盘要点…"
        aria-label="当前着法备注内容"
        @blur="saveComment"
        @keydown.ctrl.enter.prevent="saveComment"
      />

      <div class="mt-2 flex items-center justify-between gap-2">
        <span class="text-caption text-muted-foreground">Ctrl+Enter 保存</span>
        <div class="flex items-center gap-1.5">
          <Button variant="ghost" size="sm" class="h-6 px-2 text-body-sm" aria-label="恢复当前备注" @click="restoreComment">恢复</Button>
          <Button variant="ghost" size="sm" class="h-6 px-2 text-body-sm" @click="clearComment">清除</Button>
          <Button size="sm" class="h-6 px-2 text-body-sm" @click="saveComment">保存备注</Button>
        </div>
      </div>
    </template>

    <div v-else class="flex items-center justify-between gap-2">
      <div>
        <p class="text-body-sm font-semibold text-foreground">给当前局面写备注</p>
        <p class="mt-0.5 text-caption text-muted-foreground">选择一手着法或开局按钮后开始记录。</p>
      </div>
      <Button variant="outline" size="sm" class="h-7 shrink-0 px-2 text-body-sm" @click="ensureCurrentNode">开始记录</Button>
    </div>
  </section>
</template>
