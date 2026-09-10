<script setup lang="ts">
import { ref, watch } from "vue";
import { Button, Textarea } from "@/components/ui";
import { useGameStore } from "@/stores/game";
import { useManualStore } from "@/stores/manual";

const game = useGameStore();
const manual = useManualStore();
const draft = ref("");
const draftNodeId = ref<number | null>(null);

watch(
  () => manual.currentNode,
  (node) => {
    draftNodeId.value = node?.id ?? null;
    draft.value = node?.comment ?? "";
  },
  { immediate: true },
);

function ensureCurrentNode() {
  if (manual.currentNode) return true;
  return manual.selectPly(game.currentPly);
}

function saveComment() {
  if (!ensureCurrentNode() || draftNodeId.value === null) return;
  manual.updateComment(draftNodeId.value, draft.value);
}

function clearComment() {
  draft.value = "";
  saveComment();
}

function restoreComment() {
  draft.value = manual.currentNode?.comment ?? "";
}
</script>

<template>
  <section class="border-l-2 border-primary/70 bg-primary/[0.04] px-2.5 py-2" aria-label="着法备注">
    <template v-if="manual.currentNode">
      <div class="flex items-center justify-between gap-2">
        <div class="min-w-0">
          <p class="text-body-sm font-semibold text-foreground">着法备注</p>
          <p class="truncate text-caption text-muted-foreground">
            {{ manual.currentNode.mv ? manual.currentNode.chinese_notation : "起始局面" }}
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
