<script setup lang="ts">
import { computed } from "vue";
import { CATEGORY_NAMES, formatShortcut, SHORTCUT_DEFINITIONS, type ShortcutDefinition } from "@/lib/shortcuts";

const groupedShortcuts = computed(() => {
  const groups: Record<ShortcutDefinition["category"], ShortcutDefinition[]> = { game: [], board: [], workspace: [], general: [] };
  for (const item of SHORTCUT_DEFINITIONS) groups[item.category].push(item);
  return groups;
});
</script>

<template>
  <div class="flex flex-col gap-4">
    <div>
      <h4 class="text-xs font-semibold text-foreground">快捷键完整指南</h4>
      <p class="text-[11px] text-muted-foreground mt-0.5">在对弈或复盘过程中，随时按下 <kbd class="px-1.5 py-0.5 rounded border bg-muted font-mono text-[11px]">?</kbd> 或 <kbd class="px-1.5 py-0.5 rounded border bg-muted font-mono text-[11px]">F1</kbd> 即可呼出快速检索面板。</p>
    </div>
    <div class="space-y-4">
      <template v-for="(groupName, category) in CATEGORY_NAMES" :key="category">
        <div v-if="groupedShortcuts[category].length > 0" class="space-y-1.5">
          <h5 class="text-[11px] font-semibold text-muted-foreground uppercase px-0.5">{{ groupName }}</h5>
          <div class="rounded-lg border divide-y bg-card text-xs">
            <div v-for="item in groupedShortcuts[category]" :key="item.id" class="flex flex-col items-start gap-2 p-2.5 sm:flex-row sm:items-center sm:justify-between">
              <div class="min-w-0 pr-3"><span class="block truncate font-medium text-foreground">{{ item.title }}</span><span v-if="item.description" class="block truncate text-[11px] text-muted-foreground">{{ item.description }}</span></div>
              <div class="shrink-0 flex items-center gap-1"><kbd v-for="(segment, index) in item.keys.split(' / ')" :key="index" class="px-1.5 py-0.5 rounded border bg-muted font-mono text-body-sm shadow-xs">{{ formatShortcut(segment) }}</kbd></div>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
