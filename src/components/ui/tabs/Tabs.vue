<script setup lang="ts">
import { provide, ref, watch } from "vue";
import type { HTMLAttributes } from "vue";
import { cn } from "@/lib/utils";
import { TabsInjectionKey } from "./tabs-context";

const props = withDefaults(
  defineProps<{ modelValue?: string; class?: HTMLAttributes["class"] }>(),
  { modelValue: "" }
);
const emit = defineEmits<{
  (e: "change", value: string): void;
  (e: "update:modelValue", value: string): void;
}>();

const active = ref(props.modelValue);

watch(
  () => props.modelValue,
  (val) => {
    if (val !== undefined && val !== active.value) {
      active.value = val;
    }
  }
);

provide(TabsInjectionKey, {
  active: () => active.value,
  setActive: (value: string) => {
    active.value = value;
    emit("change", value);
    emit("update:modelValue", value);
  },
});
</script>

<template>
  <div :class="cn('w-full', props.class)"><slot /></div>
</template>
