<script setup lang="ts">
import { inject } from "vue";
import { cn } from "@/lib/utils";
import { TabsInjectionKey } from "./tabs-context";

const props = defineProps<{ value: string }>();
const ctx = inject(TabsInjectionKey)!;
</script>

<template>
  <button
    type="button"
    :class="cn(
      'inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 py-1 text-sm font-medium transition-all focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50',
      ctx.active() === props.value
        ? 'bg-background text-foreground shadow'
        : 'text-muted-foreground hover:text-foreground'
    )"
    @click="ctx.setActive(props.value)"
  >
    <slot />
  </button>
</template>
