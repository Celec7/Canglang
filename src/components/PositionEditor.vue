<script setup lang="ts">
import { computed, onMounted } from "vue";
import {
  AlertCircle,
  ArrowLeft,
  Eraser,
  RotateCcw,
  Trash2,
} from "@lucide/vue";
import ChessBoard from "./ChessBoard.vue";
import { Button, Separator } from "@/components/ui";
import {
  BLACK_PIECES,
  RED_PIECES,
  usePositionEditor,
} from "@/composables/usePositionEditor";
import type { BoardViewModel, Turn } from "@/lib/chess";

const emit = defineEmits<{ (e: "close"): void }>();
const editor = usePositionEditor();
const { draftBoard, turn, selectedPiece, pickedSquare, undoStack, error, validating } = editor;

const editorView = computed<BoardViewModel>(() => ({
  board: draftBoard.value,
  turn: turn.value,
  selection: pickedSquare.value,
}));

onMounted(editor.open);

function setTurn(t: Turn) {
  editor.setTurn(t);
}

async function confirm() {
  if (await editor.confirm()) emit("close");
}

const activeToolLabel = computed(() => {
  if (pickedSquare.value) {
    const [r, c] = pickedSquare.value;
    const p = draftBoard.value[r]?.[c];
    return `已拾起盘上【${p ?? "棋子"}】，点击目标格以挪动（再次点击原位取消）`;
  }
  if (selectedPiece.value === "ERASE") {
    return "已提起橡皮擦：点击要清除的棋子（右键亦可直接删除）";
  }
  if (selectedPiece.value) {
    const item = [...RED_PIECES, ...BLACK_PIECES].find((p) => p.code === selectedPiece.value);
    return `已提【${item?.side === "red" ? "红" : "黑"}${item?.label}】，点击棋盘目标格放入（单次单颗，放完手空）`;
  }
  return "当前手上无子：可直接点击盘上棋子挪动，或从提子槽取一颗棋子放置";
});
</script>

<template>
  <div class="flex h-full flex-col bg-background text-foreground select-none">
    <!-- 顶部控制条：取消、先手选择、快捷功能与确认开局 -->
    <header class="flex h-10 shrink-0 items-center justify-between border-b px-4 bg-card/60 backdrop-blur-xs">
      <div class="flex items-center gap-2.5">
        <Button variant="ghost" size="sm" class="h-8 gap-1 px-2.5 text-xs text-muted-foreground hover:text-foreground" @click="emit('close')">
          <ArrowLeft class="size-4" />
          <span>返回</span>
        </Button>
        <Separator orientation="vertical" class="h-4" />
        <h2 class="text-xs font-semibold text-foreground tracking-tight">自定义局面摆子</h2>
      </div>

      <!-- 先手选择 -->
      <div class="flex items-center gap-2 text-xs">
        <span class="text-muted-foreground">行棋先手：</span>
        <div class="flex items-center rounded-md border bg-muted/40 p-0.5">
          <button
            type="button"
            class="rounded px-2.5 py-0.5 text-xs transition-colors"
            :class="turn === 'red' ? 'bg-background font-semibold text-side-red-fg shadow-xs' : 'text-muted-foreground hover:text-foreground'"
            @click="setTurn('red')"
          >
            红方先手
          </button>
          <button
            type="button"
            class="rounded px-2.5 py-0.5 text-xs transition-colors"
            :class="turn === 'black' ? 'bg-background font-semibold text-foreground shadow-xs' : 'text-muted-foreground hover:text-foreground'"
            @click="setTurn('black')"
          >
            黑方先手
          </button>
        </div>
      </div>

      <!-- 快捷排局模板与提交 -->
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" class="h-8 text-xs" @click="editor.resetInitial">初始排局</Button>
        <Button variant="outline" size="sm" class="h-8 text-xs" @click="editor.clearBoard">
          <Trash2 class="size-3 mr-1" />
          清空
        </Button>
        <Button variant="outline" size="sm" class="h-8 text-xs" :disabled="undoStack.length === 0" @click="editor.undo">
          <RotateCcw class="size-3 mr-1" />
          撤销
        </Button>
        <Separator orientation="vertical" class="h-4" />
        <Button size="sm" class="h-8 text-xs font-semibold" :disabled="validating" @click="confirm">
          {{ validating ? "校验中…" : "确认开局" }}
        </Button>
      </div>
    </header>

    <!-- 主编辑区：上方黑子提子槽 + 中间超大主棋盘 + 下方红子提子槽 -->
    <main class="flex flex-1 flex-col items-center justify-between p-3 min-h-0 overflow-hidden relative">
      <!-- 错误校验提示卡片（绝对浮动在右上角） -->
      <div
        v-if="error"
        class="absolute top-4 right-6 z-20 flex items-center gap-2 rounded-lg border border-destructive/30 bg-destructive/10 px-3.5 py-2 text-xs text-destructive shadow-md backdrop-blur-xs max-w-md animate-in fade-in zoom-in-95"
      >
        <AlertCircle class="size-4 shrink-0" />
        <span>局面不合规：{{ error }}</span>
      </div>

      <!-- 顶部提子槽：黑方棋子 -->
      <div class="flex items-center gap-1.5 py-1 px-3.5 rounded-full bg-card/90 border shadow-xs">
        <span class="text-body-sm font-medium text-muted-foreground mr-1">黑方:</span>
        <button
          v-for="piece in BLACK_PIECES"
          :key="piece.code"
          type="button"
          class="flex size-8 items-center justify-center rounded-full border text-xs font-bold transition-transform hover:scale-110 active:scale-95 text-foreground"
          :class="[
            selectedPiece === piece.code && !pickedSquare
              ? 'border-primary bg-accent ring-2 ring-primary text-primary shadow-xs'
              : 'border-border bg-background hover:bg-accent',
          ]"
          :aria-label="`黑${piece.label}`"
          @click="editor.selectPiece(piece.code)"
        >
          {{ piece.label }}
        </button>

        <Separator orientation="vertical" class="h-4 mx-1" />

        <!-- 橡皮擦 -->
        <button
          type="button"
          class="flex size-8 items-center justify-center rounded-full border text-xs transition-colors hover:bg-accent"
          :class="[
            selectedPiece === 'ERASE' && !pickedSquare
              ? 'border-primary bg-destructive/15 text-destructive ring-2 ring-destructive shadow-xs'
              : 'border-border text-muted-foreground hover:text-foreground',
          ]"
          title="橡皮擦：点击盘上棋子清除（或直接右键点击任意棋子）"
          @click="editor.selectPiece('ERASE')"
        >
          <Eraser class="size-3.5" />
        </button>
      </div>

      <!-- 中间超大主棋盘（占满 100% 可用视高与宽度） -->
      <div class="flex flex-1 min-h-0 w-full items-center justify-center overflow-hidden p-1">
        <ChessBoard
          class="h-full w-full max-h-[76vh]"
          :view="editorView"
          editor-mode
          @square-click="editor.handleSquareClick"
          @square-right-click="editor.removeAt"
        />
      </div>

      <!-- 底部提子槽：红方棋子 -->
      <div class="flex items-center gap-1.5 py-1 px-3.5 rounded-full bg-card/90 border shadow-xs">
        <span class="text-body-sm font-medium text-side-red-fg mr-1">红方:</span>
        <button
          v-for="piece in RED_PIECES"
          :key="piece.code"
          type="button"
          class="flex size-8 items-center justify-center rounded-full border text-xs font-bold transition-transform hover:scale-110 active:scale-95 text-side-red-fg"
          :class="[
            selectedPiece === piece.code && !pickedSquare
              ? 'border-primary bg-accent ring-2 ring-primary text-primary shadow-xs'
              : 'border-border bg-background hover:bg-accent',
          ]"
          :aria-label="`红${piece.label}`"
          @click="editor.selectPiece(piece.code)"
        >
          {{ piece.label }}
        </button>

        <Separator orientation="vertical" class="h-4 mx-1" />

        <!-- 橡皮擦 -->
        <button
          type="button"
          class="flex size-8 items-center justify-center rounded-full border text-xs transition-colors hover:bg-accent"
          :class="[
            selectedPiece === 'ERASE' && !pickedSquare
              ? 'border-primary bg-destructive/15 text-destructive ring-2 ring-destructive shadow-xs'
              : 'border-border text-muted-foreground hover:text-foreground',
          ]"
          title="橡皮擦：点击盘上棋子清除（或直接右键点击任意棋子）"
          @click="editor.selectPiece('ERASE')"
        >
          <Eraser class="size-3.5" />
        </button>
      </div>

      <!-- 底部工具提示状态栏 -->
      <div class="mt-1 flex items-center justify-between w-full max-w-xl px-2 text-body-sm text-muted-foreground">
        <span class="font-medium text-foreground/80">{{ activeToolLabel }}</span>
        <span>提示：右键点击棋盘可直接删除棋子</span>
      </div>
    </main>
  </div>
</template>
