<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import { cn } from "@/lib/utils";

const props = defineProps<{
  defaultValue?: string | number;
  modelValue?: string | number;
  class?: HTMLAttributes["class"];
}>();

const emits = defineEmits<{
  (e: "update:modelValue", payload: string | number): void;
}>();

function onInput(event: Event) {
  const target = event.target as HTMLInputElement;
  emits("update:modelValue", target.value);
}
</script>

<template>
  <input
    :value="props.modelValue ?? props.defaultValue"
    :class="
      cn(
        'flex h-8 w-full rounded-md border border-input bg-background px-3 py-1 text-xs shadow-xs transition-colors file:border-0 file:bg-transparent file:text-xs file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50',
        props.class
      )
    "
    @input="onInput"
  />
</template>
