<script setup lang="ts">
import { inject } from "vue";
import { cn } from "@/lib/utils";
import { TabsInjectionKey } from "./tabs-context";

const props = defineProps<{ value: string }>();
const ctx = inject(TabsInjectionKey)!;
</script>

<template>
  <div v-if="ctx.active() === props.value" :class="cn('mt-2', $attrs.class as string)">
    <slot />
  </div>
</template>
