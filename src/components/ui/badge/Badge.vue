<script setup lang="ts">
import { computed } from "vue";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-md border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none",
  {
    variants: {
      variant: {
        default: "border-transparent bg-primary text-primary-foreground",
        secondary: "border-transparent bg-secondary text-secondary-foreground",
        destructive: "border-transparent bg-destructive text-destructive-foreground",
        outline: "text-foreground",
      },
    },
    defaultVariants: { variant: "default" },
  }
);

type BadgeVariant = VariantProps<typeof badgeVariants>["variant"];

const props = withDefaults(defineProps<{ variant?: BadgeVariant }>(), { variant: "default" });
const classes = computed(() => cn(badgeVariants({ variant: props.variant })));
</script>

<template>
  <span :class="classes"><slot /></span>
</template>
