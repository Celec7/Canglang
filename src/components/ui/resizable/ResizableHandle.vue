<script setup lang="ts">
import { computed } from "vue";
import type { HTMLAttributes } from "vue";
import {
  SplitterResizeHandle,
  type SplitterResizeHandleEmits,
  type SplitterResizeHandleProps,
  useForwardPropsEmits,
} from "radix-vue";
import { GripVertical } from "@lucide/vue";
import { cn } from "@/lib/utils";

const props = defineProps<
  SplitterResizeHandleProps & {
    class?: HTMLAttributes["class"];
    withHandle?: boolean;
  }
>();
const emits = defineEmits<SplitterResizeHandleEmits>();

const delegatedProps = computed(() => {
  const { class: _, withHandle: __, ...delegated } = props;
  return delegated;
});

const forwarded = useForwardPropsEmits(delegatedProps, emits);
</script>

<template>
  <SplitterResizeHandle
    v-bind="forwarded"
    :class="
      cn(
        'relative flex w-1.5 items-center justify-center bg-transparent transition-colors hover:bg-primary/20 cursor-col-resize focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring [&[data-orientation=vertical]]:h-1.5 [&[data-orientation=vertical]]:w-full [&[data-orientation=vertical]]:cursor-row-resize',
        props.class
      )
    "
  >
    <template v-if="props.withHandle">
      <div class="z-10 flex h-4 w-3 items-center justify-center rounded-xs border bg-border shadow-xs">
        <GripVertical class="size-2.5 text-muted-foreground" />
      </div>
    </template>
  </SplitterResizeHandle>
</template>
