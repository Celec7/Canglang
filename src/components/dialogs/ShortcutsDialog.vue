<script setup lang="ts">
import { computed, ref } from "vue";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  Input,
  ScrollArea,
} from "@/components/ui";
import {
  CATEGORY_NAMES,
  formatShortcut,
  SHORTCUT_DEFINITIONS,
  type ShortcutDefinition,
} from "@/lib/shortcuts";
import { Keyboard, Search } from "@lucide/vue";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ (e: "update:open", value: boolean): void }>();

const searchQuery = ref("");

const filteredList = computed(() => {
  const query = searchQuery.value.trim().toLowerCase();
  if (!query) return SHORTCUT_DEFINITIONS;
  return SHORTCUT_DEFINITIONS.filter(
    (item) =>
      item.title.toLowerCase().includes(query) ||
      item.keys.toLowerCase().includes(query) ||
      (item.description && item.description.toLowerCase().includes(query))
  );
});

const groupedShortcuts = computed(() => {
  const groups: Record<ShortcutDefinition["category"], ShortcutDefinition[]> = {
    game: [],
    board: [],
    workspace: [],
    general: [],
  };

  for (const item of filteredList.value) {
    groups[item.category].push(item);
  }

  return groups;
});
</script>

<template>
  <Dialog :open="props.open" @update:open="emit('update:open', $event)">
    <DialogContent class="sm:max-w-[560px] p-0 overflow-hidden">
      <DialogHeader class="px-5 pt-5 pb-3 border-b">
        <div class="flex items-center gap-2">
          <div class="flex size-7 items-center justify-center rounded-md bg-primary/10 text-primary">
            <Keyboard class="size-4" />
          </div>
          <div>
            <DialogTitle class="text-base font-semibold">键盘快捷键速查</DialogTitle>
            <DialogDescription class="text-xs text-muted-foreground mt-0.5">
              使用键盘快捷键高效掌控对局、棋盘光标与引擎推算
            </DialogDescription>
          </div>
        </div>
        <div class="relative mt-3">
          <Search class="absolute left-2.5 top-2.5 size-3.5 text-muted-foreground" />
          <Input
            v-model="searchQuery"
            type="text"
            placeholder="搜索功能或快捷键..."
            class="h-8.5 pl-8 text-xs bg-muted/30"
          />
        </div>
      </DialogHeader>

      <ScrollArea class="max-h-[60vh] px-5 py-3">
        <div class="space-y-4">
          <template v-for="(groupName, cat) in CATEGORY_NAMES" :key="cat">
            <div v-if="groupedShortcuts[cat].length > 0" class="space-y-1.5">
              <h4 class="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase px-1">
                {{ groupName }}
              </h4>
              <div class="rounded-lg border bg-card/50 divide-y divide-border/60">
                <div
                  v-for="item in groupedShortcuts[cat]"
                  :key="item.id"
                  class="flex items-center justify-between py-2 px-3 text-xs"
                >
                  <div class="min-w-0 pr-3">
                    <span class="font-medium text-foreground block truncate">{{ item.title }}</span>
                    <span v-if="item.description" class="text-[11px] text-muted-foreground truncate block">
                      {{ item.description }}
                    </span>
                  </div>
                  <div class="shrink-0 flex items-center gap-1">
                    <kbd
                      v-for="(seg, idx) in item.keys.split(' / ')"
                      :key="idx"
                      class="inline-flex h-5 items-center justify-center rounded border bg-muted/80 px-1.5 font-mono text-[11px] font-medium text-foreground shadow-xs"
                    >
                      {{ formatShortcut(seg) }}
                    </kbd>
                  </div>
                </div>
              </div>
            </div>
          </template>

          <div v-if="filteredList.length === 0" class="py-8 text-center text-xs text-muted-foreground">
            未找到匹配的快捷键
          </div>
        </div>
      </ScrollArea>
    </DialogContent>
  </Dialog>
</template>
