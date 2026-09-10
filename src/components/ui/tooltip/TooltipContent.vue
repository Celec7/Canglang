<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import {
  TooltipContent,
  type TooltipContentEmits,
  type TooltipContentProps,
  TooltipPortal,
  useForwardPropsEmits,
} from "radix-vue";
import { cn } from "@/lib/utils";

const props = withDefaults(
  defineProps<TooltipContentProps & { class?: HTMLAttributes["class"] }>(),
  {
    sideOffset: 4,
  }
);
const emits = defineEmits<TooltipContentEmits>();

const forwarded = useForwardPropsEmits(props, emits);
</script>

<template>
  <TooltipPortal>
    <TooltipContent
      v-bind="forwarded"
      :class="
        cn(
          'tooltip-content z-50 overflow-hidden rounded-md border bg-popover px-2.5 py-1 text-xs text-popover-foreground shadow-md',
          props.class
        )
      "
    >
      <slot />
    </TooltipContent>
  </TooltipPortal>
</template>
