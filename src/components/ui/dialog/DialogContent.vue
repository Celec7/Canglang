<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import {
  DialogClose,
  DialogContent,
  type DialogContentEmits,
  type DialogContentProps,
  DialogOverlay,
  DialogPortal,
  useForwardPropsEmits,
} from "radix-vue";
import { X } from "@lucide/vue";
import { cn } from "@/lib/utils";

const props = defineProps<DialogContentProps & { class?: HTMLAttributes["class"] }>();
const emits = defineEmits<DialogContentEmits>();

const forwarded = useForwardPropsEmits(props, emits);
</script>

<template>
  <DialogPortal>
    <DialogOverlay
      class="dialog-overlay fixed inset-0 z-50 bg-black/60 backdrop-blur-xs"
    />
    <div class="fixed inset-0 z-50 flex items-center justify-center p-4 overflow-y-auto pointer-events-none">
      <DialogContent
        v-bind="forwarded"
        :class="
          cn(
            'dialog-content pointer-events-auto relative z-50 grid w-full max-w-lg gap-4 border bg-background p-6 shadow-xl sm:rounded-xl md:w-full',
            props.class
          )
        "
      >
        <slot />

        <DialogClose
          class="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none"
        >
          <X class="size-4 text-muted-foreground" />
          <span class="sr-only">关闭</span>
        </DialogClose>
      </DialogContent>
    </div>
  </DialogPortal>
</template>
