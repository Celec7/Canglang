<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import { ScrollAreaScrollbar, type ScrollAreaScrollbarProps, ScrollAreaThumb } from "radix-vue";
import { cn } from "@/lib/utils";

const props = withDefaults(
  defineProps<ScrollAreaScrollbarProps & { class?: HTMLAttributes["class"] }>(),
  {
    orientation: "vertical",
  }
);
</script>

<template>
  <ScrollAreaScrollbar
    v-bind="props"
    :class="
      cn(
        'flex touch-none select-none transition-colors',
        orientation === 'vertical' && 'h-full w-2 border-l border-l-transparent p-[1px]',
        orientation === 'horizontal' && 'h-2 flex-col border-t border-t-transparent p-[1px]',
        props.class
      )
    "
  >
    <ScrollAreaThumb class="relative flex-1 rounded-full bg-border hover:bg-muted-foreground/50 transition-colors" />
  </ScrollAreaScrollbar>
</template>
