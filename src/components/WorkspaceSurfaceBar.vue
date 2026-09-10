<script setup lang="ts">
import { nextTick, ref } from "vue";
import type { WorkspaceSurface } from "@/composables/useWorkspaceLayout";

defineProps<{
  activeSurface: WorkspaceSurface | null;
  showMovesSurface?: boolean;
}>();

const emit = defineEmits<{
  (event: "toggle", surface: WorkspaceSurface): void;
  (event: "close"): void;
}>();

const movesTrigger = ref<HTMLButtonElement | null>(null);
const analysisTrigger = ref<HTMLButtonElement | null>(null);

function focusSurface(surface: WorkspaceSurface) {
  void nextTick(() => {
    const trigger = surface === "moves" ? movesTrigger.value : analysisTrigger.value;
    trigger?.focus();
  });
}

defineExpose({ focusSurface });
</script>

<template>
  <div class="workspace-surface-bar" role="toolbar" aria-label="辅助工作区">
    <button
      v-if="showMovesSurface !== false"
      ref="movesTrigger"
      type="button"
      class="workspace-surface-button"
      :class="activeSurface === 'moves' ? 'workspace-surface-button--active' : ''"
      :aria-pressed="activeSurface === 'moves'"
      @click="emit('toggle', 'moves')"
    >
      着法记录
    </button>
    <button
      ref="analysisTrigger"
      type="button"
      class="workspace-surface-button"
      :class="activeSurface === 'analysis' ? 'workspace-surface-button--active' : ''"
      :aria-pressed="activeSurface === 'analysis'"
      @click="emit('toggle', 'analysis')"
    >
      实时分析
    </button>
    <button
      v-if="activeSurface"
      type="button"
      class="workspace-surface-button workspace-surface-button--close"
      aria-label="关闭辅助工作区"
      @click="emit('close')"
    >
      关闭
    </button>
  </div>
</template>
