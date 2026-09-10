<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import {
  ScrollAreaCorner,
  ScrollAreaRoot,
  type ScrollAreaRootProps,
  ScrollAreaViewport,
} from "radix-vue";
import { cn } from "@/lib/utils";
import ScrollBar from "./ScrollBar.vue";

const props = withDefaults(
  defineProps<ScrollAreaRootProps & { class?: HTMLAttributes["class"] }>(),
  {
    type: "hover",
  }
);
</script>

<template>
  <ScrollAreaRoot :type="props.type" :class="cn('relative overflow-hidden', props.class)">
    <ScrollAreaViewport class="size-full rounded-[inherit]">
      <slot />
    </ScrollAreaViewport>
    <ScrollBar />
    <ScrollAreaCorner />
  </ScrollAreaRoot>
</template>
