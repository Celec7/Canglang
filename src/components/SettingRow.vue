<script setup lang="ts">
import { onMounted, onUpdated, ref, useId } from "vue";

defineProps<{
  label: string;
  description?: string;
}>();

const rowRoot = ref<HTMLElement | null>(null);
const labelId = useId();

function associateControls() {
  const root = rowRoot.value;
  if (!root) return;

  root.querySelectorAll<HTMLElement>('input, textarea, select, [role="combobox"], [role="switch"], [role="slider"], [role="textbox"]').forEach((control) => {
    if (!control.hasAttribute("aria-label") && !control.hasAttribute("aria-labelledby")) {
      control.setAttribute("aria-labelledby", labelId);
    }
  });
}

onMounted(associateControls);
onUpdated(associateControls);
</script>

<template>
  <div ref="rowRoot" class="flex flex-col items-start gap-2.5 border-b py-3 last:border-b-0 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
    <div class="min-w-0">
      <div :id="labelId" class="text-sm font-medium">{{ label }}</div>
      <div v-if="$slots.description" class="mt-1 text-xs leading-5 text-muted-foreground"><slot name="description" /></div>
      <div v-else-if="description" class="mt-1 text-xs leading-5 text-muted-foreground">{{ description }}</div>
    </div>
    <div class="w-full min-w-0 sm:w-auto sm:shrink-0">
      <slot />
    </div>
  </div>
</template>
