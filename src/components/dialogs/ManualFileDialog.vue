<script setup lang="ts">
import { ref } from "vue";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Textarea,
} from "@/components/ui";
import { FolderOpen } from "@lucide/vue";
import { useManualStore } from "@/stores/manual";
import { useToast } from "@/composables/useToast";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ (e: "update:open", value: boolean): void }>();

const manual = useManualStore();
const { show } = useToast();

const loadPath = ref("");
const savePath = ref("");
const xqfSavePath = ref("");

async function loadFile() {
  if (!loadPath.value.trim()) return;
  if (!manual.confirmDiscard()) return;
  await manual.load(loadPath.value.trim());
  if (manual.error) {
    show(`载入棋谱失败: ${manual.error}`);
  } else {
    const applied = await manual.applyToGame();
    if (!applied) {
      show(`载入主线失败: ${manual.error ?? "棋谱中包含无法应用的走法"}`);
      return;
    }
    show("棋谱已成功载入");
    emit("update:open", false);
  }
}

async function pickLoadFile() {
  try {
    const path = await manual.pickFile("open");
    if (path) loadPath.value = path;
  } catch (cause) {
    show(`选择棋谱失败: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
}

async function applyLoadedManual() {
  const applied = await manual.applyToGame();
  if (!applied && manual.error) {
    show(`载入主线失败: ${manual.error}`);
  }
}

function clearManual() {
  if (!manual.confirmDiscard()) return;
  manual.clear();
}

async function saveFile() {
  if (!savePath.value.trim()) return;
  await manual.save(savePath.value.trim());
  if (manual.error) {
    show(`保存棋谱失败: ${manual.error}`);
  } else {
    show("棋谱已成功保存为 .pgn");
  }
}

async function pickPgnSaveFile() {
  try {
    const path = await manual.pickFile("save");
    if (path) savePath.value = path.endsWith(".pgn") ? path : `${path}.pgn`;
  } catch (cause) {
    show(`选择 PGN 保存位置失败: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
}

async function exportPgnText() {
  await manual.exportPgn();
  if (manual.error) {
    show(`生成 PGN 文本失败: ${manual.error}`);
  } else if (manual.generatedPgn) {
    show("已成功生成 PGN 文本");
  }
}

async function saveXqfFile() {
  if (!xqfSavePath.value.trim()) return;
  await manual.saveXqf(xqfSavePath.value.trim(), 10);
  if (manual.error) {
    show(`保存 XQF 失败: ${manual.error}`);
  } else {
    show("棋谱已成功保存为规范化 XQF v10");
  }
}

async function pickXqfSaveFile() {
  try {
    const path = await manual.pickFile("save");
    if (path) xqfSavePath.value = path.endsWith(".xqf") ? path : `${path}.xqf`;
  } catch (cause) {
    show(`选择 XQF 保存位置失败: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
}
</script>

<template>
  <Dialog :open="props.open" @update:open="emit('update:open', $event)">
    <DialogContent class="max-w-lg">
      <DialogHeader>
        <DialogTitle>棋谱导入与导出 (PGN / XQF)</DialogTitle>
        <DialogDescription>载入本地棋谱，或将当前对局保存为 PGN 与规范化 XQF v10。</DialogDescription>
      </DialogHeader>

      <div class="flex flex-col gap-4 py-2 text-xs">
        <!-- 载入本地棋谱 -->
        <div class="flex flex-col gap-1.5">
          <label class="font-medium text-foreground">打开棋谱文件 (.pgn / .xqf)</label>
          <div class="flex flex-wrap gap-2">
            <Input v-model="loadPath" placeholder="输入文件绝对路径..." class="min-w-0 flex-1" />
            <Button variant="outline" size="sm" aria-label="浏览棋谱文件" @click="pickLoadFile"><FolderOpen class="size-3.5" />浏览</Button>
            <Button size="sm" :disabled="manual.loading" @click="loadFile">
              {{ manual.loading ? "载入中" : "打开" }}
            </Button>
          </div>
        </div>

        <!-- 当前载入棋谱元数据信息 -->
        <div v-if="manual.manual" class="rounded-md border bg-muted/40 p-3 leading-relaxed text-muted-foreground">
          <div class="font-medium text-foreground text-sm">{{ manual.manual.title }}</div>
          <div class="mt-1 grid grid-cols-2 gap-1 text-body-sm">
            <div>红方：{{ manual.manual.red_player ?? "未标注" }}</div>
            <div>黑方：{{ manual.manual.black_player ?? "未标注" }}</div>
            <div>赛事：{{ manual.manual.event_name ?? "未标注" }}</div>
            <div>日期：{{ manual.manual.date ?? "未标注" }}</div>
          </div>
          <div class="mt-2 flex gap-2">
            <Button size="sm" variant="secondary" class="h-6 text-body-sm" @click="applyLoadedManual">载入主线至棋盘</Button>
            <Button size="sm" variant="outline" class="h-6 text-body-sm" @click="clearManual">清除棋谱</Button>
          </div>
        </div>

        <!-- 保存当前棋谱 -->
        <div class="flex flex-col gap-1.5 border-t pt-3">
          <label class="font-medium text-foreground">保存当前对局为 PGN 文件</label>
          <div class="flex flex-wrap gap-2">
            <Input v-model="savePath" placeholder="保存为 /path/to/game.pgn" class="min-w-0 flex-1" />
            <Button variant="outline" size="sm" aria-label="选择 PGN 保存位置" @click="pickPgnSaveFile"><FolderOpen class="size-3.5" />浏览</Button>
            <Button variant="outline" size="sm" @click="saveFile">保存</Button>
          </div>
        </div>

        <!-- 规范化 XQF v10 导出 -->
        <div class="flex flex-col gap-1.5 border-t pt-3">
          <label class="font-medium text-foreground">保存当前对局为 XQF v10 文件</label>
          <div class="flex flex-wrap gap-2">
            <Input v-model="xqfSavePath" placeholder="保存为 /path/to/game.xqf" class="min-w-0 flex-1" />
            <Button variant="outline" size="sm" aria-label="选择 XQF 保存位置" @click="pickXqfSaveFile"><FolderOpen class="size-3.5" />浏览</Button>
            <Button variant="outline" size="sm" @click="saveXqfFile">保存</Button>
          </div>
          <p class="text-caption text-muted-foreground">生成未加密规范化文件；含变例或 GBK 无法表示的文字时会提示失败。</p>
        </div>

        <!-- 快速导出 PGN 文本 -->
        <div class="flex flex-col gap-1.5 border-t pt-3">
          <div class="flex items-center justify-between">
            <label class="font-medium text-foreground">导出 PGN 纯文本</label>
            <Button size="sm" variant="secondary" class="h-6 text-body-sm" @click="exportPgnText">生成文本</Button>
          </div>
          <Textarea
            v-if="manual.generatedPgn"
            :model-value="manual.generatedPgn"
            readonly
            rows="4"
            class="font-mono text-caption leading-relaxed"
          />
        </div>
      </div>

      <DialogFooter>
        <DialogClose as-child>
          <Button variant="outline" size="sm">关闭</Button>
        </DialogClose>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
