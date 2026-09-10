<script setup lang="ts">
import { ref, watch } from "vue";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Textarea,
} from "@/components/ui";
import { useGameStore } from "@/stores/game";
import { useToast } from "@/composables/useToast";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ (e: "update:open", value: boolean): void }>();

const game = useGameStore();
const { show } = useToast();
const fenText = ref(game.fen ?? "");
const copied = ref(false);

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      fenText.value = game.fen ?? "";
    }
  }
);

async function importFen() {
  const trimmed = fenText.value.trim();
  if (!trimmed) return;
  try {
    if (!(await game.newGame(trimmed))) return;
    show("已载入自定义 FEN 局面");
    emit("update:open", false);
  } catch (cause) {
    show(cause instanceof Error ? cause.message : String(cause));
  }
}

async function copyFen() {
  if (!navigator.clipboard) {
    show("当前环境不支持剪贴板");
    return;
  }
  try {
    if (!game.capabilities.use_fen.enabled || !game.fen) {
      show("当前对局不支持 FEN");
      return;
    }
    await navigator.clipboard.writeText(game.fen);
    copied.value = true;
    show("FEN 已复制到剪贴板");
    setTimeout(() => {
      copied.value = false;
    }, 1500);
  } catch (cause) {
    show(cause instanceof Error ? cause.message : "复制 FEN 失败");
  }
}

async function resetInitial() {
  try {
    if (!(await game.newGame())) return;
    fenText.value = game.fen ?? "";
    show("已重置为标准开局局面");
    emit("update:open", false);
  } catch (cause) {
    show(cause instanceof Error ? cause.message : String(cause));
  }
}
</script>

<template>
  <Dialog :open="props.open" @update:open="emit('update:open', $event)">
    <DialogContent class="max-w-md">
      <DialogHeader>
        <DialogTitle>FEN 局面工具</DialogTitle>
        <DialogDescription>查看、复制当前局面的 FEN 串，或粘贴外部 FEN 快速加载对局。</DialogDescription>
      </DialogHeader>

      <div class="flex flex-col gap-2 py-2">
        <label class="text-xs font-medium text-foreground">FEN 字符串</label>
        <Textarea
          v-model="fenText"
          rows="3"
          class="font-mono text-body-sm leading-relaxed select-all"
          placeholder="输入 2 段式中国象棋 FEN..."
        />
      </div>

      <DialogFooter class="flex flex-wrap items-center justify-between gap-2 sm:justify-between">
        <div class="flex items-center gap-1.5">
          <Button variant="outline" size="sm" @click="copyFen">
            {{ copied ? "已复制" : "复制 FEN" }}
          </Button>
          <Button variant="ghost" size="sm" class="text-muted-foreground" @click="resetInitial">
            重置为初始局
          </Button>
        </div>

        <div class="flex items-center gap-1.5">
          <DialogClose as-child>
            <Button variant="outline" size="sm">取消</Button>
          </DialogClose>
          <Button size="sm" @click="importFen">导入局面</Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
