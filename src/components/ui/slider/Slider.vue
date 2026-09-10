<script setup lang="ts">
import { computed } from "vue";
import {
  SliderRange,
  SliderRoot,
  SliderThumb,
  SliderTrack,
  useForwardPropsEmits,
  type SliderRootEmits,
  type SliderRootProps,
} from "radix-vue";
import { cn } from "@/lib/utils";

const props = defineProps<SliderRootProps & { class?: string }>();
const emits = defineEmits<SliderRootEmits>();

const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props;
  return delegated;
});

const forwarded = useForwardPropsEmits(delegatedProps, emits);
</script>

<template>
  <SliderRoot
    v-bind="forwarded"
    :class="
      cn(
        'relative flex w-full touch-none select-none items-center',
        props.class
      )
    "
  >
    <SliderTrack class="relative h-1.5 w-full grow overflow-hidden rounded-full bg-secondary">
      <SliderRange class="absolute h-full bg-primary" />
    </SliderTrack>
    <SliderThumb
      v-for="(_, key) in (modelValue ?? defaultValue ?? [0])"
      :key="key"
      class="block size-4 rounded-full border border-primary/50 bg-background shadow-xs transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50"
    />
  </SliderRoot>
</template>
