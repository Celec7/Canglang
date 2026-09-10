<script setup lang="ts">
import { ref } from "vue";
import type { CloudBookMode } from "@/bindings";
import { useBookStore } from "@/stores/book";
import { usePreferencesStore } from "@/stores/preferences";
import { useToast } from "@/composables/useToast";
import { Badge, Button, Input, Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue, Separator, Switch } from "@/components/ui";
import { Cloud, FolderOpen, Plus, Trash2 } from "@lucide/vue";
import SettingRow from "../SettingRow.vue";

const book = useBookStore();
const preferences = usePreferencesStore();
const { show } = useToast();
const newBookPath = ref("");

async function addOpeningBook(pathToAdd?: string) {
  const target = (pathToAdd ?? newBookPath.value).trim();
  if (!target) return;
  try {
    await book.load(target);
    preferences.addOpeningBookPath(target);
    newBookPath.value = "";
    show("开局库已载入并持久化保存");
  } catch (cause) {
    show(`载入开局库失败: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
}

async function pickOpeningBookFile() {
  try {
    const result = await book.pickFile();
    if (result) await addOpeningBook(result);
  } catch (cause) {
    show(`选择开局库失败: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
}

async function removeOpeningBook(path: string) {
  try {
    await book.unload(path);
    preferences.removeOpeningBookPath(path);
    show("开局库已卸载");
  } catch (cause) {
    show(`卸载开局库失败: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="space-y-2.5">
      <div class="flex items-center justify-between"><div class="flex items-center gap-1.5"><Cloud class="size-4 text-sky-500" /><h4 class="text-xs font-semibold text-foreground">象棋云库 (chessdb.cn)</h4></div><Badge :variant="preferences.cloudBookEnabled ? 'default' : 'outline'" class="text-[10px] h-4">{{ preferences.cloudBookEnabled ? "已启用" : "未开启" }}</Badge></div>
      <div class="rounded-lg border bg-card p-3 space-y-3 text-xs">
        <SettingRow label="启用在线云库查询" description="在对弈与复盘中实时检索云端百万级大师开局与残局着法"><Switch aria-label="启用在线云库查询" :checked="preferences.cloudBookEnabled" @update:checked="preferences.setCloudBookEnabled($event)" /></SettingRow>
        <SettingRow label="云端协同策略" description="指定本地 .bh 文件与在线云库的结合调用方式">
        <Select :model-value="preferences.cloudBookMode" :disabled="!preferences.cloudBookEnabled" @update:model-value="preferences.setCloudBookMode($event as CloudBookMode)"><SelectTrigger class="h-8 w-full text-xs sm:w-48" aria-label="云端协同策略"><SelectValue /></SelectTrigger><SelectContent><SelectGroup><SelectItem value="hybrid">协同模式 (本地优先，脱谱查云)</SelectItem><SelectItem value="merge">合并模式 (本地与云端合并)</SelectItem><SelectItem value="cloud_only">纯云库模式 (仅查在线云端)</SelectItem><SelectItem value="local_only">纯离线模式 (仅查本地 .bh)</SelectItem></SelectGroup></SelectContent></Select>
        </SettingRow>
      </div>
    </div>
    <Separator />
    <div class="space-y-2.5">
      <div class="flex items-center justify-between"><h4 class="text-xs font-semibold text-foreground">本地开局库 (.bh 格式)</h4><Badge variant="secondary" class="text-[10px] h-4">已载入 {{ book.loaded.length }} 个库</Badge></div>
      <div class="flex flex-wrap items-center gap-2"><Input v-model="newBookPath" aria-label="本地开局库路径" placeholder="输入本地 .bh 开局库路径" class="h-8 min-w-0 flex-1 text-xs font-mono sm:min-w-48" @keyup.enter="addOpeningBook()" /><Button variant="outline" size="sm" class="h-8 shrink-0 gap-1 text-xs" @click="pickOpeningBookFile"><FolderOpen class="size-3.5" />选择文件...</Button><Button size="sm" class="h-8 shrink-0 gap-1 text-xs" @click="addOpeningBook()"><Plus class="size-3.5" />添加</Button></div>
      <div v-if="preferences.openingBookPaths.length > 0" class="rounded-lg border divide-y bg-card text-xs"><div v-for="path in preferences.openingBookPaths" :key="path" class="flex items-center justify-between p-2.5 gap-2"><div class="flex flex-col min-w-0"><span class="font-medium text-foreground truncate">{{ path.split(/[/\\]/).pop() }}</span><span class="text-[10px] font-mono text-muted-foreground truncate">{{ path }}</span></div><div class="flex items-center gap-2 shrink-0"><Badge variant="outline" class="text-[10px]">自动加载</Badge><Button variant="ghost" size="icon" class="size-6 text-destructive hover:bg-destructive/10" title="移除开局库" @click="removeOpeningBook(path)"><Trash2 class="size-3.5" /></Button></div></div></div>
      <div v-else class="rounded-lg border border-dashed p-4 text-center text-xs text-muted-foreground">尚未配置本地 .bh 开局库。可在此添加常用的本地库文件。</div>
    </div>
  </div>
</template>
