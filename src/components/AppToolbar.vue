<script setup lang="ts">
import {
  Columns2,
  MoreHorizontal,
  FileCode2,
  FilePlus2,
  Keyboard,
  Moon,
  PanelRight,
  Settings,
  SlidersHorizontal,
  Sun,
  Volume2,
  VolumeX,
} from "@lucide/vue";
import { nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { Button, Separator, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui";
import { startWindowDragging } from "@/lib/window";
import type { ThemeMode } from "@/lib/preferences";
import { useEngineStore } from "@/stores/engine";
import { usePreferencesStore } from "@/stores/preferences";
import WindowControls from "./WindowControls.vue";
import logoUrl from "@/assets/logo.svg";

const props = defineProps<{
  theme: ThemeMode;
  showAnalysis: boolean;
  showMoveList: boolean;
}>();

const emit = defineEmits<{
  (e: "toggleTheme"): void;
  (e: "openSettings"): void;
  (e: "openEngineConfig"): void;
  (e: "openFen"): void;
  (e: "openManual"): void;
  (e: "toggleAnalysis"): void;
  (e: "toggleMoveList"): void;
  (e: "newGame"): void;
  (e: "openShortcuts"): void;
}>();

const engine = useEngineStore();
const preferences = usePreferencesStore();

type ToolbarMenu = "engine" | "record" | "more";

const openToolbarMenu = ref<ToolbarMenu | null>(null);
const engineMenuTrigger = ref<HTMLButtonElement | null>(null);
const recordMenuTrigger = ref<HTMLButtonElement | null>(null);
const moreMenuTrigger = ref<HTMLButtonElement | null>(null);

function toggleToolbarMenu(menu: ToolbarMenu) {
  openToolbarMenu.value = openToolbarMenu.value === menu ? null : menu;
}

function closeToolbarMenu(restoreFocus = false) {
  const menu = openToolbarMenu.value;
  openToolbarMenu.value = null;

  if (restoreFocus && menu) {
    void nextTick(() => {
      const trigger = menu === "engine"
        ? engineMenuTrigger.value
        : menu === "record"
          ? recordMenuTrigger.value
          : moreMenuTrigger.value;
      trigger?.focus();
    });
  }
}

function selectEngineAction(action: "off" | "analysis" | "red" | "black") {
  if (action === "off") engine.setAnalysisEnabled(false);
  if (action === "analysis") engine.setAnalysisOnly();
  if (action === "red") engine.toggleAutoMoveRed();
  if (action === "black") engine.toggleAutoMoveBlack();
  closeToolbarMenu(true);
}

function emitToolbarAction(event: "newGame" | "openManual" | "openFen" | "openEngineConfig" | "toggleTheme" | "openShortcuts" | "openSettings") {
  closeToolbarMenu(true);
  if (event === "newGame") emit("newGame");
  if (event === "openManual") emit("openManual");
  if (event === "openFen") emit("openFen");
  if (event === "openEngineConfig") emit("openEngineConfig");
  if (event === "toggleTheme") emit("toggleTheme");
  if (event === "openShortcuts") emit("openShortcuts");
  if (event === "openSettings") emit("openSettings");
}

function toggleToolbarSound() {
  preferences.setSoundEnabled(!preferences.soundEnabled);
  closeToolbarMenu(true);
}

function handleToolbarPointerdown(event: PointerEvent) {
  const target = event.target;
  if (target instanceof Element && !target.closest(".toolbar-menu")) {
    closeToolbarMenu();
  }
}

function handleToolbarKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && openToolbarMenu.value) {
    event.preventDefault();
    closeToolbarMenu(true);
  }
}

onMounted(() => {
  document.addEventListener("pointerdown", handleToolbarPointerdown, true);
  document.addEventListener("keydown", handleToolbarKeydown, true);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", handleToolbarPointerdown, true);
  document.removeEventListener("keydown", handleToolbarKeydown, true);
});

function dragWindow(event: MouseEvent) {
  if (event.button === 0) void startWindowDragging();
}
</script>

<template>
  <header class="relative z-50 flex h-11 min-w-0 shrink-0 items-center justify-between border-b bg-background/95 px-3 backdrop-blur-sm select-none">
    <!-- 应用品牌与全局动作 -->
    <div class="toolbar-brand flex shrink-0 items-center gap-2 pr-1" aria-label="应用信息">
      <img
        :src="logoUrl"
        alt="沧浪象棋"
        class="size-6 rounded-md shadow-xs select-none pointer-events-none object-contain"
      />
      <span class="toolbar-brand-name text-sm font-semibold tracking-tight text-foreground font-sans">Canglang</span>
    </div>

    <Separator orientation="vertical" class="mx-1 hidden h-4.5 sm:block" />

    <div class="toolbar-primary-actions flex shrink-0 items-center gap-1">
      <Tooltip>
        <TooltipTrigger as-child>
          <Button variant="ghost" size="sm" class="toolbar-new-game h-7.5 gap-1.5 px-2.5 text-xs font-medium rounded-md" @click="emit('newGame')">
            <FilePlus2 class="size-3.5 text-muted-foreground" />
            <span class="hidden sm:inline">新对局</span>
          </Button>
        </TooltipTrigger>
        <TooltipContent>开始新的对局 (Ctrl+N)</TooltipContent>
      </Tooltip>

      <div class="toolbar-menu toolbar-record-menu">
        <button
          ref="recordMenuTrigger"
          type="button"
          class="toolbar-menu__summary"
          aria-label="棋谱工具"
          aria-controls="toolbar-record-menu"
          :aria-expanded="openToolbarMenu === 'record'"
          @click="toggleToolbarMenu('record')"
        >
          <FileCode2 class="size-3.5" />
          <span class="hidden sm:inline">棋谱</span>
        </button>
        <div v-if="openToolbarMenu === 'record'" id="toolbar-record-menu" class="toolbar-menu__content toolbar-record-menu__content" role="menu">
          <button type="button" class="toolbar-menu__item" role="menuitem" @click="emitToolbarAction('openManual')">棋谱导入 / 导出</button>
          <button type="button" class="toolbar-menu__item" role="menuitem" @click="emitToolbarAction('openFen')">FEN 局面工具</button>
        </div>
      </div>
    </div>

    <!-- 中间窗口拖拽安全区（清爽沉浸，无干扰浅字） -->
    <div
      class="toolbar-drag-region flex h-full min-w-0 flex-1 cursor-default items-center justify-center"
      data-tauri-drag-region
      aria-hidden="true"
      @mousedown="dragWindow"
    />

    <!-- 右侧引擎快速模式与面板切换 -->
    <div class="flex min-w-0 shrink-0 items-center gap-1.5">
      <!-- 引擎控制按钮组：关闭分析 | 仅分析 | 执红 | 执黑（红/黑可同时启用） -->
      <div class="toolbar-menu toolbar-engine-menu flex lg:hidden">
        <button
          ref="engineMenuTrigger"
          type="button"
          class="toolbar-menu__summary"
          aria-label="引擎控制"
          aria-controls="toolbar-engine-menu"
          :aria-expanded="openToolbarMenu === 'engine'"
          @click="toggleToolbarMenu('engine')"
        >
          引擎
        </button>
        <div v-if="openToolbarMenu === 'engine'" id="toolbar-engine-menu" class="toolbar-menu__content toolbar-engine-menu__content" role="menu">
          <button
            type="button"
            class="toolbar-menu__item"
            role="menuitem"
            :class="!engine.analysisEnabled ? 'toolbar-menu__item--active' : ''"
            :aria-pressed="!engine.analysisEnabled"
            aria-label="关闭分析"
            @click="selectEngineAction('off')"
          >
            关闭分析
          </button>
          <button
            type="button"
            class="toolbar-menu__item"
            role="menuitem"
            :class="engine.analysisEnabled && engine.isAnalysisOnly ? 'toolbar-menu__item--active' : ''"
            :aria-pressed="engine.analysisEnabled && engine.isAnalysisOnly"
            aria-label="仅分析"
            @click="selectEngineAction('analysis')"
          >
            仅分析
          </button>
          <button
            type="button"
            class="toolbar-menu__item"
            role="menuitem"
            :class="engine.analysisEnabled && engine.autoMoveRed ? 'toolbar-menu__item--red' : ''"
            :aria-pressed="engine.autoMoveRed"
            aria-label="执红"
            @click="selectEngineAction('red')"
          >
            执红
          </button>
          <button
            type="button"
            class="toolbar-menu__item"
            role="menuitem"
            :class="engine.analysisEnabled && engine.autoMoveBlack ? 'toolbar-menu__item--active' : ''"
            :aria-pressed="engine.autoMoveBlack"
            aria-label="执黑"
            @click="selectEngineAction('black')"
          >
            执黑
          </button>
        </div>
      </div>

      <div class="hidden items-center rounded-lg border bg-muted/40 p-0.5 text-xs lg:flex">
        <button
          type="button"
          class="rounded-md px-2.5 py-1 text-xs transition-colors"
          :class="
            !engine.analysisEnabled
              ? 'bg-background font-semibold shadow-xs text-muted-foreground ring-1 ring-border'
              : 'text-muted-foreground/70 hover:text-foreground'
          "
          title="关闭引擎思考与自动走子"
          @click="engine.setAnalysisEnabled(false)"
        >
          关闭分析
        </button>
        <button
          type="button"
          class="rounded-md px-2.5 py-1 text-xs transition-colors"
          :class="
            engine.analysisEnabled && engine.isAnalysisOnly
              ? 'bg-background font-semibold shadow-xs text-primary ring-1 ring-primary/30'
              : 'text-muted-foreground hover:text-foreground'
          "
          title="电脑仅分析，不自动走子"
          @click="engine.setAnalysisOnly()"
        >
          仅分析
        </button>
        <button
          type="button"
          class="rounded-md px-2.5 py-1 text-xs transition-colors"
          :class="
            engine.analysisEnabled && engine.autoMoveRed
              ? 'bg-background font-semibold shadow-xs text-side-red-fg ring-1 ring-side-red-fg/40'
              : 'text-muted-foreground hover:text-foreground'
          "
          title="电脑执红走棋"
          @click="engine.toggleAutoMoveRed()"
        >
          执红
        </button>
        <button
          type="button"
          class="rounded-md px-2.5 py-1 text-xs transition-colors"
          :class="
            engine.analysisEnabled && engine.autoMoveBlack
              ? 'bg-background font-semibold shadow-xs text-foreground ring-1 ring-foreground/30'
              : 'text-muted-foreground hover:text-foreground'
          "
          title="电脑执黑走棋"
          @click="engine.toggleAutoMoveBlack()"
        >
          执黑
        </button>
      </div>

      <!-- 引擎配置入口 -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="toolbar-optional-control size-7.5 rounded-md" aria-label="引擎配置" @click="emit('openEngineConfig')">
            <SlidersHorizontal class="size-3.5" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>引擎参数与深度配置</TooltipContent>
      </Tooltip>

      <Separator orientation="vertical" class="h-4.5 mx-0.5" />

      <!-- 中栏分析监视区收折 Toggle -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            variant="ghost"
            size="icon"
            class="toolbar-workspace-control size-7.5 rounded-md"
            :class="props.showAnalysis ? 'text-primary bg-primary/10' : 'text-muted-foreground'"
            :aria-pressed="props.showAnalysis"
            aria-label="切换中栏分析 (Ctrl+1 / Ctrl+B)"
            @click="emit('toggleAnalysis')"
          >
            <Columns2 class="size-3.5" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ props.showAnalysis ? "收起中栏分析区 (Ctrl+1 / Ctrl+B)" : "展开中栏分析区 (Ctrl+1 / Ctrl+B)" }}</TooltipContent>
      </Tooltip>

      <!-- 右栏棋谱列表收折 Toggle -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            variant="ghost"
            size="icon"
            class="toolbar-workspace-control size-7.5 rounded-md"
            :class="props.showMoveList ? 'text-primary bg-primary/10' : 'text-muted-foreground'"
            :aria-pressed="props.showMoveList"
            aria-label="切换右栏着法 (Ctrl+2 / Ctrl+J)"
            @click="emit('toggleMoveList')"
          >
            <PanelRight class="size-3.5" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ props.showMoveList ? "收起右侧着法列表 (Ctrl+2 / Ctrl+J)" : "展开右侧着法列表 (Ctrl+2 / Ctrl+J)" }}</TooltipContent>
      </Tooltip>

      <Separator orientation="vertical" class="h-4.5 mx-0.5" />

      <!-- 音效开关 -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            variant="ghost"
            size="icon"
            class="toolbar-optional-control size-7.5 rounded-md"
            :class="preferences.soundEnabled ? 'text-foreground' : 'text-muted-foreground/60'"
            :aria-label="preferences.soundEnabled ? '静音 (M)' : '开启音效 (M)'"
            @click="preferences.setSoundEnabled(!preferences.soundEnabled)"
          >
            <Volume2 v-if="preferences.soundEnabled" class="size-3.5" />
            <VolumeX v-else class="size-3.5 text-muted-foreground/60" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ preferences.soundEnabled ? "静音走子音效 (M)" : "开启走子音效 (M)" }}</TooltipContent>
      </Tooltip>

      <!-- 主题切换 -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="toolbar-optional-control size-7.5 rounded-md" :aria-label="props.theme === 'dark' ? '切换浅色模式' : '切换深色模式'" @click="emit('toggleTheme')">
            <Sun v-if="props.theme === 'dark'" class="size-3.5" />
            <Moon v-else class="size-3.5" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{{ props.theme === 'dark' ? "切换为浅色模式" : "切换为深色模式" }}</TooltipContent>
      </Tooltip>

      <!-- 快捷键速查 -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="toolbar-optional-control size-7.5 rounded-md" aria-label="快捷键速查 (? / F1)" @click="emit('openShortcuts')">
            <Keyboard class="size-3.5" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>快捷键速查 (? / F1)</TooltipContent>
      </Tooltip>

      <!-- 设置 -->
      <Tooltip>
        <TooltipTrigger as-child>
          <Button variant="ghost" size="icon" class="toolbar-optional-control size-7.5 rounded-md" aria-label="全局设置 (Ctrl+,)" @click="emit('openSettings')">
            <Settings class="size-3.5" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>偏好设置 (Ctrl+,)</TooltipContent>
      </Tooltip>

      <div class="toolbar-menu toolbar-more-menu flex lg:hidden">
        <button
          ref="moreMenuTrigger"
          type="button"
          class="toolbar-menu__summary"
          aria-label="更多工具"
          aria-controls="toolbar-more-menu"
          :aria-expanded="openToolbarMenu === 'more'"
          @click="toggleToolbarMenu('more')"
        >
          <MoreHorizontal class="size-4" />
        </button>
        <div v-if="openToolbarMenu === 'more'" id="toolbar-more-menu" class="toolbar-menu__content toolbar-more-menu__content" role="menu">
          <button type="button" class="toolbar-menu__item" role="menuitem" @click="emitToolbarAction('newGame')">新对局</button>
          <button type="button" class="toolbar-menu__item" role="menuitem" @click="emitToolbarAction('openEngineConfig')">引擎配置</button>
          <button type="button" class="toolbar-menu__item" role="menuitem" @click="toggleToolbarSound">
            {{ preferences.soundEnabled ? "静音" : "开启音效" }}
          </button>
          <button type="button" class="toolbar-menu__item" role="menuitem" @click="emitToolbarAction('toggleTheme')">
            {{ props.theme === 'dark' ? "浅色模式" : "深色模式" }}
          </button>
          <button type="button" class="toolbar-menu__item" role="menuitem" @click="emitToolbarAction('openShortcuts')">快捷键</button>
          <button type="button" class="toolbar-menu__item" role="menuitem" @click="emitToolbarAction('openSettings')">设置</button>
        </div>
      </div>

      <WindowControls class="shrink-0" />
    </div>
  </header>
</template>

<style scoped>
.toolbar-menu {
  position: relative;
}

.toolbar-menu__summary {
  display: inline-flex;
  min-height: 30px;
  align-items: center;
  justify-content: center;
  gap: 0.25rem;
  border-radius: 0.375rem;
  padding: 0.375rem 0.5rem;
  color: hsl(var(--muted-foreground));
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  touch-action: manipulation;
}

.toolbar-menu__summary:hover,
.toolbar-menu__summary[aria-expanded="true"] {
  background: hsl(var(--accent));
  color: hsl(var(--foreground));
}

.toolbar-menu__summary:focus-visible {
  outline: 2px solid hsl(var(--ring));
  outline-offset: 2px;
}

.toolbar-menu__content {
  position: absolute;
  top: calc(100% + 0.375rem);
  right: 0;
  z-index: 70;
  display: flex;
  min-width: 8rem;
  flex-direction: column;
  gap: 0.125rem;
  border: 1px solid hsl(var(--border));
  border-radius: 0.5rem;
  background: hsl(var(--popover));
  padding: 0.375rem;
  color: hsl(var(--popover-foreground));
  box-shadow: 0 12px 30px hsl(var(--foreground) / 0.18);
}

.toolbar-engine-menu__content {
  right: auto;
  left: 0;
}

.toolbar-menu__item {
  min-height: 2rem;
  border-radius: 0.375rem;
  padding: 0.375rem 0.625rem;
  text-align: left;
  white-space: nowrap;
  color: hsl(var(--muted-foreground));
  font-size: 0.75rem;
}

.toolbar-menu__item:hover,
.toolbar-menu__item:focus-visible {
  background: hsl(var(--accent));
  color: hsl(var(--foreground));
  outline: none;
}

.toolbar-menu__item--active {
  background: hsl(var(--primary) / 0.12);
  color: hsl(var(--primary));
  font-weight: 600;
}

.toolbar-menu__item--red {
  background: color-mix(in srgb, var(--side-red-fg) 12%, transparent);
  color: var(--side-red-fg);
  font-weight: 600;
}

@media (max-width: 1023px) {
  .toolbar-optional-control {
    display: none;
  }
}

@media (max-width: 639px) {
  .toolbar-brand-name,
  .toolbar-primary-actions .toolbar-new-game,
  .toolbar > .separator {
    display: none;
  }

  .toolbar-primary-actions {
    display: flex;
  }

  .toolbar-brand {
    gap: 0;
    padding-right: 0;
  }

  .toolbar-drag-region {
    flex-basis: 0;
  }
}
</style>
