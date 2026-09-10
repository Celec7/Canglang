<script setup lang="ts">
import type { PreviewSnapshot } from "@/bindings";
import { Button } from "@/components/ui";

defineProps<{ snapshot: PreviewSnapshot; moveCount: number }>();
const emit = defineEmits<{ apply: []; clear: [] }>();
</script>

<template>
  <div class="flex flex-wrap items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-2.5 py-2 text-[11px]">
    <span class="font-medium text-primary">正在预览变例</span>
    <span class="text-muted-foreground">第 {{ moveCount }} 步 · {{ snapshot.in_check ? "将军" : "普通局面" }}</span>
    <span v-if="snapshot.rule_explanation" class="text-amber-600">{{ snapshot.rule_explanation.message }}</span>
    <div class="ml-auto flex gap-1">
      <Button size="sm" class="h-6 px-2 text-[11px]" @click="emit('apply')">应用到棋盘</Button>
      <Button size="sm" variant="ghost" class="h-6 px-2 text-[11px]" @click="emit('clear')">返回</Button>
    </div>
  </div>
</template>
