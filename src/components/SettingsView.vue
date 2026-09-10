<script setup lang="ts">
import { ref, watch } from "vue";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui";
import { BookOpen, Cpu, Info, Keyboard, Palette, Scale } from "@lucide/vue";
import { usePreferencesStore } from "@/stores/preferences";
import AppearanceTab from "./settings/AppearanceTab.vue";
import RulesTab from "./settings/RulesTab.vue";
import EngineTab from "./settings/EngineTab.vue";
import BookTab from "./settings/BookTab.vue";
import ShortcutsTab from "./settings/ShortcutsTab.vue";
import AboutTab from "./settings/AboutTab.vue";

export type SettingsTab = "appearance" | "rules" | "engine" | "book" | "shortcuts" | "about";

const props = withDefaults(
  defineProps<{ open?: boolean; initialTab?: SettingsTab }>(),
  { open: true, initialTab: "appearance" }
);

const emit = defineEmits<{
  (event: "close"): void;
  (event: "update:open", value: boolean): void;
}>();

const preferences = usePreferencesStore();
const activeTab = ref<SettingsTab>(props.initialTab);
const navItems = [
  { id: "appearance", label: "界面与棋盘", icon: Palette },
  { id: "rules", label: "对局与规则", icon: Scale },
  { id: "engine", label: "引擎与分析", icon: Cpu },
  { id: "book", label: "开局库管理", icon: BookOpen },
  { id: "shortcuts", label: "快捷键指南", icon: Keyboard },
  { id: "about", label: "关于应用", icon: Info },
] as const;

watch(() => props.initialTab, (tab) => {
  if (tab) activeTab.value = tab;
});

function onOpenChange(value: boolean) {
  emit("update:open", value);
  if (!value) emit("close");
}
</script>

<template>
  <Dialog :open="props.open" @update:open="onOpenChange">
    <DialogContent class="max-w-3xl w-[calc(100vw-1rem)] max-w-[calc(100vw-1rem)] min-w-0 gap-0 overflow-hidden p-0 sm:w-full sm:rounded-xl">
      <DialogHeader class="px-4 sm:px-5 py-3 border-b bg-card/60">
        <DialogTitle class="text-sm font-semibold">偏好设置</DialogTitle>
        <DialogDescription class="text-xs text-muted-foreground">自定义界面外观、棋规判定、引擎高级参数与本地开局库。</DialogDescription>
      </DialogHeader>

      <div class="settings-layout flex h-[min(640px,80vh)] min-h-0 min-w-0 flex-col divide-y select-none sm:flex-row sm:divide-x sm:divide-y-0">
        <aside class="w-full min-w-0 shrink-0 overflow-x-auto bg-muted/20 p-2 sm:w-44 sm:overflow-visible">
          <nav class="flex flex-row sm:flex-col gap-1" aria-label="设置分类">
            <button v-for="item in navItems" :key="item.id" type="button" class="flex shrink-0 items-center gap-2.5 px-3 py-2 rounded-md text-xs font-medium transition-colors text-left" :class="activeTab === item.id ? 'bg-background text-primary shadow-xs font-semibold' : 'text-muted-foreground hover:bg-accent/60 hover:text-foreground'" :aria-current="activeTab === item.id ? 'page' : undefined" @click="activeTab = item.id">
              <component :is="item.icon" class="size-4 shrink-0" /><span>{{ item.label }}</span>
            </button>
          </nav>
          <div v-if="preferences.locationInfo" class="hidden sm:block rounded-lg border bg-card/60 p-2 text-[10px] space-y-0.5">
            <div class="flex items-center gap-1.5 font-medium"><span class="size-1.5 rounded-full" :class="preferences.locationInfo.isPortable ? 'bg-emerald-500' : 'bg-primary'" /><span class="text-foreground font-semibold">{{ preferences.locationInfo.isPortable ? "便携模式" : "标准模式" }}</span></div>
            <p class="text-muted-foreground truncate font-mono text-[9px]" :title="preferences.locationInfo.filePath">{{ preferences.locationInfo.isPortable ? "exe 同级 config.json" : "系统标准配置目录" }}</p>
          </div>
        </aside>

        <main class="min-w-0 max-w-full flex-1 overflow-x-hidden overflow-y-auto overscroll-contain p-3 sm:p-5" :aria-label="`${navItems.find((item) => item.id === activeTab)?.label ?? '设置'}设置`">
          <AppearanceTab v-if="activeTab === 'appearance'" />
          <RulesTab v-else-if="activeTab === 'rules'" />
          <EngineTab v-else-if="activeTab === 'engine'" :open="props.open && activeTab === 'engine'" />
          <BookTab v-else-if="activeTab === 'book'" />
          <ShortcutsTab v-else-if="activeTab === 'shortcuts'" />
          <AboutTab v-else-if="activeTab === 'about'" />
        </main>
      </div>
    </DialogContent>
  </Dialog>
</template>
