<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import {
  SelectItem,
  SelectItemIndicator,
  type SelectItemProps,
  SelectItemText,
  useForwardProps,
} from "radix-vue";
import { Check } from "@lucide/vue";
import { cn } from "@/lib/utils";

const props = defineProps<SelectItemProps & { class?: HTMLAttributes["class"] }>();

const forwarded = useForwardProps(props);
</script>

<template>
  <SelectItem
    v-bind="forwarded"
    :class="
      cn(
        'relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-2 pr-8 text-xs outline-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50',
        props.class
      )
    "
  >
    <span class="absolute right-2 flex size-3.5 items-center justify-center">
      <SelectItemIndicator>
        <Check class="size-3.5" />
      </SelectItemIndicator>
    </span>
    <SelectItemText>
      <slot />
    </SelectItemText>
  </SelectItem>
</template>
