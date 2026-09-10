<script setup lang="ts">
import { useToast } from "@/composables/useToast";

const { messages } = useToast();
</script>

<template>
  <div class="pointer-events-none fixed inset-x-0 bottom-6 z-[100] flex justify-center px-4" aria-live="polite" aria-atomic="true">
    <TransitionGroup name="toast" tag="div" class="flex max-w-full flex-col items-center gap-2">
      <p
        v-for="toast in messages"
        :key="toast.id"
        class="max-w-md break-all rounded-full bg-foreground px-4 py-2 text-xs font-medium text-background shadow-xl ring-1 ring-background/20 text-center"
      >
        {{ toast.message }}
      </p>
    </TransitionGroup>
  </div>
</template>
