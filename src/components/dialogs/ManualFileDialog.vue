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
import { useGameStore } from "@/stores/game";
import { useToast } from "@/composables/useToast";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ (e: "update:open", value: boolean): void }>();

const manual = useManualStore();
const game = useGameStore();
const { show } = useToast();

const loadPath = ref("");
const savePath = ref("");
const xqfSavePath = ref("");
const jieqiLoadPath = ref("");
const jieqiSavePath = ref("");
const jieqiPublicPath = ref("");

async function loadFile() {
  if (!loadPath.value.trim()) return;
  if (!(await manual.confirmDiscard())) return;
  if (await manual.openXiangqi(loadPath.value.trim())) {
    show("棋谱已成功载入");
    emit("update:open", false);
  } else {
    show(`载入棋谱失败: ${manual.error ?? "棋谱中包含无法应用的走法"}`);
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

async function clearManual() {
  if (!(await manual.confirmDiscard())) return;
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
    const path = await manual.pickFile("save_xqf");
    if (path) xqfSavePath.value = path.endsWith(".xqf") ? path : `${path}.xqf`;
  } catch (cause) {
    show(`选择 XQF 保存位置失败: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
}

async function pickJieqiLoadFile() {
  try {
    const path = await manual.pickFile("open_jieqi");
    if (path) jieqiLoadPath.value = path;
  } catch (cause) {
    show(`选择揭棋文档失败: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
}

async function loadJieqiFile() {
  if (!jieqiLoadPath.value.trim() || !(await manual.confirmDiscard())) return;
  if (await manual.openJieqi(jieqiLoadPath.value.trim())) {
    show("揭棋文档已成功载入");
    emit("update:open", false);
  } else {
    show(`载入揭棋文档失败: ${manual.error ?? "未知错误"}`);
  }
}

async function pickJieqiSaveFile(kind: "private_game" | "public_replay") {
  try {
    const path = await manual.pickFile(kind === "private_game" ? "save_jieqi_private" : "save_jieqi_public");
    if (!path) return;
    const normalized = path.endsWith(".cjq") ? path : `${path}.cjq`;
    if (kind === "private_game") jieqiSavePath.value = normalized;
    else jieqiPublicPath.value = normalized;
  } catch (cause) {
    show(`选择揭棋保存位置失败: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
}

async function saveJieqi(kind: "private_game" | "public_replay") {
  const path = kind === "private_game" ? jieqiSavePath.value : jieqiPublicPath.value;
  if (!path.trim()) return;
  const succeeded = await manual.saveJieqi(path.trim(), kind);
  if (!succeeded) {
    show(`保存揭棋文档失败: ${manual.error ?? "未知错误"}`);
  } else if (kind === "private_game") {
    show("私有续局已保存；文件包含未揭身份，请勿公开分享");
  } else {
    show("公开回放已导出；文件不包含未揭身份");
  }
}
</script>

<template>
  <Dialog :open="props.open" @update:open="emit('update:open', $event)">
    <DialogContent class="max-w-lg">
      <DialogHeader>
        <DialogTitle>{{ game.variant === "jieqi" ? "揭棋文档" : "棋谱导入与导出 (PGN / XQF)" }}</DialogTitle>
        <DialogDescription>
          {{ game.variant === "jieqi" ? "打开 .cjq，保存可续局私有文件，或导出不含未揭身份的公开回放。" : "载入本地棋谱，或将当前对局保存为 PGN 与规范化 XQF v10。" }}
        </DialogDescription>
      </DialogHeader>

      <div v-if="game.variant !== 'jieqi'" class="flex flex-col gap-4 py-2 text-xs">
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

        <div class="flex flex-col gap-1.5 border-t pt-3">
          <label class="font-medium text-foreground">切换并打开揭棋文档 (.cjq)</label>
          <div class="flex flex-wrap gap-2">
            <Input v-model="jieqiLoadPath" placeholder="输入文件绝对路径..." class="min-w-0 flex-1" />
            <Button variant="outline" size="sm" aria-label="浏览揭棋文档" @click="pickJieqiLoadFile"><FolderOpen class="size-3.5" />浏览</Button>
            <Button variant="outline" size="sm" :disabled="manual.loading" @click="loadJieqiFile">打开</Button>
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

      <div v-else class="flex max-h-[70vh] flex-col gap-4 overflow-y-auto py-2 pr-1 text-xs">
        <div class="flex flex-col gap-1.5">
          <label class="font-medium text-foreground">打开揭棋文档 (.cjq)</label>
          <div class="flex flex-wrap gap-2">
            <Input v-model="jieqiLoadPath" placeholder="输入文件绝对路径..." class="min-w-0 flex-1" />
            <Button variant="outline" size="sm" aria-label="浏览揭棋文档" @click="pickJieqiLoadFile"><FolderOpen class="size-3.5" />浏览</Button>
            <Button size="sm" :disabled="manual.loading" @click="loadJieqiFile">{{ manual.loading ? "载入中" : "打开" }}</Button>
          </div>
        </div>

        <div class="flex flex-col gap-1.5 border-t pt-3">
          <label class="font-medium text-foreground">切换并打开中国象棋棋谱 (.pgn / .xqf)</label>
          <div class="flex flex-wrap gap-2">
            <Input v-model="loadPath" placeholder="输入文件绝对路径..." class="min-w-0 flex-1" />
            <Button variant="outline" size="sm" aria-label="浏览中国象棋棋谱" @click="pickLoadFile"><FolderOpen class="size-3.5" />浏览</Button>
            <Button variant="outline" size="sm" :disabled="manual.loading" @click="loadFile">打开</Button>
          </div>
        </div>

        <div class="grid grid-cols-1 gap-2 border-t pt-3 sm:grid-cols-2">
          <label class="flex flex-col gap-1">标题<Input v-model="manual.jieqiMetadata.title" aria-label="揭棋标题" /></label>
          <label class="flex flex-col gap-1">日期<Input v-model="manual.jieqiMetadata.date" aria-label="揭棋日期" /></label>
          <label class="flex flex-col gap-1">红方<Input v-model="manual.jieqiMetadata.red_player" aria-label="揭棋红方" /></label>
          <label class="flex flex-col gap-1">黑方<Input v-model="manual.jieqiMetadata.black_player" aria-label="揭棋黑方" /></label>
          <label class="flex flex-col gap-1 sm:col-span-2">赛事<Input v-model="manual.jieqiMetadata.event_name" aria-label="揭棋赛事" /></label>
        </div>

        <div v-if="game.capabilities.save_private.enabled" class="flex flex-col gap-1.5 border-t pt-3">
          <label class="font-medium text-foreground">保存私有续局</label>
          <div class="flex flex-wrap gap-2">
            <Input v-model="jieqiSavePath" placeholder="保存为 /path/to/game.cjq" class="min-w-0 flex-1" />
            <Button variant="outline" size="sm" aria-label="选择私有揭棋保存位置" @click="pickJieqiSaveFile('private_game')"><FolderOpen class="size-3.5" />浏览</Button>
            <Button size="sm" @click="saveJieqi('private_game')">保存</Button>
          </div>
          <p class="text-caption text-destructive">包含完整未揭身份，仅用于本人续局；不要公开分享。</p>
        </div>

        <div class="flex flex-col gap-1.5 border-t pt-3">
          <label class="font-medium text-foreground">
            {{ manual.documentKind === "public_replay" ? "保存公开回放" : "导出公开回放" }}
          </label>
          <div class="flex flex-wrap gap-2">
            <Input v-model="jieqiPublicPath" placeholder="导出为 /path/to/replay.cjq" class="min-w-0 flex-1" />
            <Button variant="outline" size="sm" aria-label="选择公开揭棋保存位置" @click="pickJieqiSaveFile('public_replay')"><FolderOpen class="size-3.5" />浏览</Button>
            <Button size="sm" variant="outline" :disabled="!game.capabilities.save_public.enabled" @click="saveJieqi('public_replay')">导出</Button>
          </div>
          <p class="text-caption text-muted-foreground">只保存主线中已经公开的揭子信息；从私有局导出时不会清除私有续局的未保存状态。</p>
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
