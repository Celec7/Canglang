<script setup lang="ts">
defineProps<{
  flow?: boolean;
}>();
</script>

<template>
  <section class="workspace-surface-panel" :class="{ 'workspace-surface-panel--flow': flow }">
    <slot />
  </section>
</template>
