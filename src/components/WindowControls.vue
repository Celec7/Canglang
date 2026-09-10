<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Copy, Minus, Square, X } from "@lucide/vue";
import {
  closeWindow,
  isWindowMaximized,
  minimizeWindow,
  toggleWindowMaximize,
} from "@/lib/window";

const maximized = ref(false);

async function refreshMaximized() {
  maximized.value = await isWindowMaximized();
}

async function toggleMaximize() {
  await toggleWindowMaximize();
  await refreshMaximized();
}

onMounted(() => {
  void refreshMaximized();
});
</script>

<template>
  <div class="flex h-full items-center gap-0.5 select-none" aria-label="窗口控制">
    <button
      type="button"
      class="window-control"
      aria-label="最小化窗口"
      title="最小化"
      @click="minimizeWindow"
    >
      <Minus class="size-3.5" />
    </button>
    <button
      type="button"
      class="window-control"
      :aria-label="maximized ? '还原窗口' : '最大化窗口'"
      :title="maximized ? '还原' : '最大化'"
      @click="toggleMaximize"
    >
      <Copy v-if="maximized" class="size-3.5" />
      <Square v-else class="size-3.5" />
    </button>
    <button
      type="button"
      class="window-control window-control-close"
      aria-label="关闭窗口"
      title="关闭"
      @click="closeWindow"
    >
      <X class="size-3.5" />
    </button>
  </div>
</template>

<style scoped>
.window-control {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 28px;
  border-radius: var(--radius);
  color: hsl(var(--muted-foreground));
  transition:
    background-color 120ms ease,
    color 120ms ease;
}

.window-control:hover {
  background: hsl(var(--accent));
  color: hsl(var(--accent-foreground));
}

.window-control:focus-visible {
  outline: 1px solid hsl(var(--ring));
  outline-offset: 1px;
}

.window-control-close:hover {
  background: hsl(var(--destructive));
  color: hsl(var(--destructive-foreground));
}
</style>
