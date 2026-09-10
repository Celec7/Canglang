<script setup lang="ts">
import { ref } from "vue";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  TooltipProvider,
} from "@/components/ui";
import MoveList from "./components/MoveList.vue";
import ReplayControls from "./components/ReplayControls.vue";
import AppToolbar from "./components/AppToolbar.vue";
import BoardWorkspace from "./components/BoardWorkspace.vue";
import PositionEditor from "./components/PositionEditor.vue";
import SettingsView, { type SettingsTab } from "./components/SettingsView.vue";
import ToastViewport from "./components/ToastViewport.vue";
import AnalysisWorkspace from "./components/AnalysisWorkspace.vue";
import WorkspaceSurfaceBar from "./components/WorkspaceSurfaceBar.vue";
import WorkspaceSurfacePanel from "./components/WorkspaceSurfacePanel.vue";
import FenDialog from "./components/dialogs/FenDialog.vue";
import ManualFileDialog from "./components/dialogs/ManualFileDialog.vue";
import ShortcutsDialog from "./components/dialogs/ShortcutsDialog.vue";
import { useGameStore } from "./stores/game";
import { usePreferencesStore } from "./stores/preferences";
import { useAppLifecycle } from "@/composables/useAppLifecycle";
import { useReplay } from "@/composables/useReplay";
import { useShortcuts } from "@/composables/useShortcuts";
import { useWorkspaceLayout, type WorkspaceSurface } from "@/composables/useWorkspaceLayout";

const game = useGameStore();
const preferences = usePreferencesStore();
const { togglePlayback } = useReplay();
const { politeMessage, assertiveMessage } = useAppLifecycle();
const {
  layoutMode,
  activeSurface,
  showAnalysis,
  showMoveList,
  toggleAnalysis,
  toggleMoveList,
  toggleSurface,
  closeSurface,
} = useWorkspaceLayout();

const fenOpen = ref(false);
const manualOpen = ref(false);
const positionEditorOpen = ref(false);
const settingsOpen = ref(false);
const shortcutsOpen = ref(false);
const settingsTab = ref<SettingsTab>("appearance");
const chessBoardRef = ref<{ selected: string | null } | null>(null);
const surfaceBarRef = ref<{ focusSurface: (surface: WorkspaceSurface) => void } | null>(null);

useShortcuts({
  onNewGame: () => void game.newGame(),
  onOpenManual: () => {
    manualOpen.value = true;
  },
  onOpenFen: () => {
    fenOpen.value = true;
  },
  onOpenSettings: () => openSettings("appearance"),
  onToggleShortcutsHelp: () => {
    shortcutsOpen.value = !shortcutsOpen.value;
  },
  onToggleAnalysisPanel: toggleAnalysis,
  onToggleMoveListPanel: toggleMoveList,
  onTogglePlayback: togglePlayback,
  hasBoardSelection: () => !!chessBoardRef.value?.selected,
});

function openSettings(tab: SettingsTab = "appearance") {
  settingsTab.value = tab;
  settingsOpen.value = true;
}

function toggleTheme() {
  preferences.setTheme(preferences.theme === "dark" ? "light" : "dark");
}

function openPositionEditor() {
  positionEditorOpen.value = true;
}

function closeSurfaceAndRestoreFocus() {
  const closingSurface = activeSurface.value;
  closeSurface();
  if (closingSurface) surfaceBarRef.value?.focusSurface(closingSurface);
}
</script>

<template>
  <TooltipProvider :delay-duration="200">
    <div class="flex h-full flex-col bg-background text-foreground">
      <AppToolbar
        :theme="preferences.theme"
        :show-analysis="showAnalysis"
        :show-move-list="showMoveList"
        @toggle-theme="toggleTheme"
        @open-settings="openSettings('appearance')"
        @open-engine-config="openSettings('engine')"
        @open-fen="fenOpen = true"
        @open-manual="manualOpen = true"
        @open-shortcuts="shortcutsOpen = true"
        @toggle-analysis="toggleAnalysis"
        @toggle-move-list="toggleMoveList"
        @new-game="game.newGame()"
      />

      <PositionEditor v-if="positionEditorOpen" @close="positionEditorOpen = false" />

      <main
        v-else
        class="workspace-grid flex min-h-0 flex-1 overflow-x-hidden p-2.5"
        :class="layoutMode === 'desktop-split' ? 'overflow-y-hidden' : 'overflow-y-auto'"
      >
        <div v-if="layoutMode === 'desktop-split'" class="min-h-0 w-full">
          <ResizablePanelGroup id="workspace-layout" direction="horizontal" class="h-full w-full">
            <ResizablePanel id="board-panel" :default-size="showAnalysis ? 54 : 78" :min-size="36">
              <BoardWorkspace ref="chessBoardRef" class="h-full w-full" @open-position="openPositionEditor" />
            </ResizablePanel>

            <template v-if="showAnalysis">
              <ResizableHandle id="handle-board-analysis" with-handle />
              <ResizablePanel id="analysis-panel" :default-size="24" :min-size="18" :max-size="45">
                <AnalysisWorkspace class="h-full w-full" />
              </ResizablePanel>
            </template>

            <template v-if="showMoveList">
              <ResizableHandle id="handle-analysis-movelist" with-handle />
              <ResizablePanel id="movelist-panel" :default-size="22" :min-size="16" :max-size="36">
                <aside class="flex h-full w-full flex-col overflow-hidden rounded-xl border bg-card shadow-xs">
                  <MoveList class="min-h-0 flex-1" />
                  <ReplayControls class="shrink-0" />
                </aside>
              </ResizablePanel>
            </template>
          </ResizablePanelGroup>
        </div>

        <div v-else class="flex min-h-full w-full flex-col gap-2">
          <div v-if="layoutMode === 'tablet-split' && showMoveList" class="flex min-h-[28rem] flex-1 gap-2">
            <section class="min-h-[28rem] min-w-0 flex-1">
              <BoardWorkspace ref="chessBoardRef" class="h-full w-full" @open-position="openPositionEditor" />
            </section>
            <aside class="flex w-[min(24vw,15rem)] min-w-[13.5rem] shrink-0 flex-col overflow-hidden rounded-xl border bg-card shadow-xs">
              <MoveList class="min-h-0 flex-1" />
              <ReplayControls class="shrink-0" />
            </aside>
          </div>
          <BoardWorkspace v-else ref="chessBoardRef" class="min-h-[min(78vh,48rem)] w-full" @open-position="openPositionEditor" />

          <WorkspaceSurfaceBar
            ref="surfaceBarRef"
            :active-surface="activeSurface"
            :show-moves-surface="layoutMode !== 'tablet-split' || !showMoveList"
            @toggle="toggleSurface"
            @close="closeSurfaceAndRestoreFocus"
          />

          <WorkspaceSurfacePanel v-if="activeSurface === 'moves'" flow class="min-h-[18rem]">
            <MoveList class="workspace-move-list-flow min-h-[18rem]" />
            <ReplayControls class="shrink-0" />
          </WorkspaceSurfacePanel>
          <WorkspaceSurfacePanel v-else-if="activeSurface === 'analysis'" flow class="min-h-[28rem]">
            <AnalysisWorkspace class="workspace-analysis-flow w-full" />
          </WorkspaceSurfacePanel>

        </div>
      </main>

      <!-- 全局模态配置弹窗系统 -->
      <FenDialog v-model:open="fenOpen" />
      <ManualFileDialog v-model:open="manualOpen" />
      <SettingsView v-model:open="settingsOpen" :initial-tab="settingsTab" />
      <ShortcutsDialog v-model:open="shortcutsOpen" />

      <!-- 无障碍实时语音广播通道（屏幕阅读器实时区域） -->
      <div class="sr-only" aria-live="polite" aria-atomic="true">
        {{ politeMessage }}
      </div>
      <div class="sr-only" aria-live="assertive" aria-atomic="true">
        {{ assertiveMessage }}
      </div>

      <ToastViewport />
    </div>
  </TooltipProvider>
</template>
