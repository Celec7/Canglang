<script setup lang="ts">
import { toRef } from "vue";
import { useEngineSettings } from "@/composables/useEngineSettings";
import { Badge, Button, Input, Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue, Separator } from "@/components/ui";
import { ChevronDown, Download, FolderOpen, Loader2, Sparkles, Trash2, X } from "@lucide/vue";
import SettingRow from "../SettingRow.vue";

const props = withDefaults(defineProps<{ open?: boolean }>(), { open: false });
const {
  engine,
  preferences,
  selectedProfileId,
  showAdvanced,
  isDownloading,
  currentProfile,
  analysis,
  builtinNotInstalled,
  builtinNeedsUpgrade,
  optionDescriptors,
  optionDescription,
  isPathOption,
  pickOptionPath,
  optionText,
  optionChecked,
  setOptionValue,
  triggerOption,
  activeDownload,
  progressPercent,
  downloadStageLabel,
  formatBytes,
  formatSpeed,
  onSelectProfile,
  deleteCurrentProfile,
  handlePickEngineFile,
  handlePickNnueFile,
  clearNnuePath,
  downloadBuiltinEngine,
  saveProfileAndSync,
  applyEngineAndRestart,
} = useEngineSettings(toRef(props, "open"));
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="flex flex-wrap items-center justify-between gap-2">
      <div class="flex items-center gap-2"><h4 class="text-xs font-semibold text-foreground">引擎配置管理</h4><Badge :variant="engine.running ? 'default' : 'outline'" class="text-[10px] h-4">{{ engine.running ? "运行中" : "未启动" }}</Badge></div>
      <div class="flex items-center gap-1.5">
        <Select :model-value="selectedProfileId" @update:model-value="onSelectProfile($event as string)"><SelectTrigger class="h-7.5 w-full text-xs sm:w-48" aria-label="引擎预设"><SelectValue placeholder="选择引擎预设" /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="profile in preferences.engineProfiles" :key="profile.id" :value="profile.id"><div class="flex items-center gap-1.5"><span class="font-medium">{{ profile.name }}</span><span v-if="profile.isBuiltin" class="text-[9px] px-1 py-0.2 rounded font-normal" :class="profile.path ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground'">{{ profile.path ? "在线 · 已就绪" : "在线 · 未安装" }}</span><span v-else class="text-[9px] px-1 py-0.2 rounded bg-muted text-muted-foreground font-normal">本地</span></div></SelectItem><SelectItem value="__new__" class="text-primary font-medium">+ 新建本地引擎配置...</SelectItem></SelectGroup></SelectContent></Select>
        <Button variant="ghost" size="icon" class="size-7.5 text-destructive hover:bg-destructive/10" :title="currentProfile.isBuiltin ? '清除已下载的内置引擎文件' : '删除当前引擎配置'" :disabled="!currentProfile.isBuiltin && preferences.engineProfiles.length <= 1" @click="deleteCurrentProfile"><Trash2 class="size-3.5" /></Button>
      </div>
    </div>

    <div v-if="builtinNotInstalled" class="rounded-lg border bg-card/60 p-3.5 flex flex-col gap-3">
      <div class="flex items-start gap-2.5"><div class="rounded-md bg-primary/10 p-1.5 shrink-0"><Sparkles class="size-4 text-primary" /></div><div class="flex flex-col gap-1 min-w-0"><div class="flex items-center gap-1.5"><span class="text-xs font-semibold text-foreground">{{ currentProfile.name }}</span><Badge variant="outline" class="text-[10px] h-4">内置在线</Badge></div><p class="text-[11px] leading-relaxed text-muted-foreground">{{ currentProfile.description || "官方预置引擎，可一键下载并自动装配。" }}</p><a v-if="currentProfile.downloadUrl" :href="currentProfile.downloadUrl" target="_blank" rel="noreferrer" class="text-[10px] text-primary hover:underline w-fit">查看官方发布来源 (GitHub)</a></div></div>
      <div v-if="isDownloading" class="flex flex-col gap-1.5"><div class="flex items-center justify-between text-[11px] text-muted-foreground"><span class="flex items-center gap-1.5 min-w-0"><Loader2 class="size-3.5 shrink-0 animate-spin text-primary" /><span class="truncate">{{ downloadStageLabel }}</span></span><span class="font-mono tabular-nums shrink-0"><template v-if="activeDownload && activeDownload.speedBytesPerSec > 0">{{ formatSpeed(activeDownload.speedBytesPerSec) }} · </template>{{ progressPercent.toFixed(1) }}%</span></div><div class="h-1.5 w-full overflow-hidden rounded-full bg-muted"><div class="h-full rounded-full bg-primary transition-[width] duration-200" :class="progressPercent > 0 ? '' : 'w-1/3 animate-pulse'" :style="progressPercent > 0 ? { width: `${progressPercent}%` } : undefined" /></div><span v-if="activeDownload" class="text-[10px] font-mono text-muted-foreground truncate">{{ formatBytes(activeDownload.downloadedBytes) }}<template v-if="activeDownload.totalBytes"> / {{ formatBytes(activeDownload.totalBytes) }}</template></span></div>
      <Button v-else size="sm" class="h-8 text-xs gap-1.5 w-fit" @click="downloadBuiltinEngine(currentProfile.id)"><Download class="size-3.5" />一键在线获取并安装（约 52 MB）</Button>
    </div>

    <div v-else class="rounded-lg border bg-card/60 p-3 flex flex-col gap-2.5">
      <div v-if="currentProfile.isBuiltin" class="flex flex-wrap items-center gap-1.5 pb-0.5"><span class="text-[11px] font-medium" :class="builtinNeedsUpgrade ? 'text-amber-600' : 'text-emerald-600'">{{ builtinNeedsUpgrade ? "有可用升级" : "✓ 已就绪" }}</span><span class="text-[10px] text-muted-foreground">已自动装配可执行文件与神经网络权重</span><Button v-if="builtinNeedsUpgrade" variant="outline" size="sm" class="ml-auto h-7 text-[11px] gap-1" :disabled="isDownloading" @click="downloadBuiltinEngine(currentProfile.id)"><Download class="size-3" />升级</Button></div>
      <SettingRow label="预设名称" description="引擎在界面中显示的友好名称"><Input v-model="currentProfile.name" placeholder="如 Pikafish, 旋风, 象棋小巫师" class="h-8 w-full text-xs sm:w-56" @change="saveProfileAndSync" /></SettingRow>
      <SettingRow label="引擎可执行文件" description="本地 UCI / UCCI 引擎程序路径"><div class="flex flex-wrap items-center gap-1.5 w-full"><Input v-model="currentProfile.path" placeholder="可执行文件绝对路径" class="flex-1 min-w-40 h-8 text-xs font-mono truncate" @change="saveProfileAndSync" /><Button variant="outline" size="sm" class="h-8 shrink-0 text-xs gap-1" @click="handlePickEngineFile"><FolderOpen class="size-3.5" />浏览</Button><Button variant="outline" size="sm" class="h-8 shrink-0 text-xs" title="启动引擎并验证 UCI/UCCI 握手" @click="applyEngineAndRestart">测试连接</Button></div></SettingRow>
      <SettingRow label="通信协议" description="支持通用 UCI 或中国象棋专有 UCCI 协议"><Select v-model="currentProfile.protocol" @update:model-value="saveProfileAndSync"><SelectTrigger class="h-8 w-full text-xs font-mono sm:w-36"><SelectValue /></SelectTrigger><SelectContent><SelectGroup><SelectItem value="auto">自动探测 (推荐)</SelectItem><SelectItem value="ucci">UCCI (通用中文象棋)</SelectItem><SelectItem value="uci">UCI (通用国际象棋接口)</SelectItem></SelectGroup></SelectContent></Select></SettingRow>
      <SettingRow label="计算线程 (Threads)" description="外部引擎使用的最大 CPU 核心线程数"><Input :model-value="currentProfile.threads ?? ''" type="number" min="1" max="128" class="h-8 w-full text-xs text-right sm:w-20" @update:model-value="currentProfile.threads = $event !== '' ? Number($event) : null" @change="saveProfileAndSync" /></SettingRow>
      <SettingRow label="置换表内存 (Hash MB)" description="用于缓存已算局面的内存置换表容量"><Input :model-value="currentProfile.hashMb ?? ''" type="number" min="16" max="32768" step="64" class="h-8 w-full text-xs text-right sm:w-24" @update:model-value="currentProfile.hashMb = $event !== '' ? Number($event) : null" @change="saveProfileAndSync" /></SettingRow>
    </div>

    <div v-if="!builtinNotInstalled" class="rounded-lg border bg-card/60"><button type="button" class="flex w-full items-center justify-between px-3 py-2 text-xs font-medium text-foreground" :aria-expanded="showAdvanced" @click="showAdvanced = !showAdvanced"><span class="flex items-center gap-1.5"><ChevronDown class="size-3.5 transition-transform" :class="showAdvanced ? 'rotate-180' : ''" />高级设置</span><span class="text-[10px] font-normal text-muted-foreground">{{ currentProfile.nnuePath ? "已挂载外置权重" : "使用引擎自带权重" }}</span></button><div v-if="showAdvanced" class="border-t p-3"><SettingRow label="外挂神经网络权重 (.nnue)" description="留空则使用引擎同目录自带的默认权重；选定后会在引擎启动时覆盖挂载"><div class="flex flex-wrap items-center gap-1.5 w-full"><Input :model-value="currentProfile.nnuePath ?? ''" placeholder="未指定（使用引擎自带权重）" class="flex-1 min-w-40 h-8 text-xs font-mono truncate" @update:model-value="currentProfile.nnuePath = typeof $event === 'string' && $event.trim() !== '' ? $event : null" @change="saveProfileAndSync" /><Button v-if="currentProfile.nnuePath" variant="ghost" size="icon" class="size-8 shrink-0" title="清除外置权重" @click="clearNnuePath"><X class="size-3.5" /></Button><Button variant="outline" size="sm" class="h-8 shrink-0 text-xs gap-1" @click="handlePickNnueFile"><FolderOpen class="size-3.5" />浏览</Button></div></SettingRow></div></div>

    <div v-if="!builtinNotInstalled && optionDescriptors.length > 0" class="rounded-lg border bg-card/60 p-3">
      <div class="mb-2">
        <div class="text-xs font-semibold">引擎选项</div>
        <p class="mt-1 text-[11px] leading-5 text-muted-foreground">以下参数来自当前引擎的 UCI/UCCI 握手声明；说明、默认值和范围均以引擎实际能力为准。</p>
      </div>
      <div class="flex flex-col gap-2">
        <SettingRow v-for="descriptor in optionDescriptors" :key="descriptor.name" :label="descriptor.name">
          <template #description>
            <p>{{ optionDescription(descriptor).summary }}</p>
            <div class="mt-1.5 flex flex-wrap gap-x-3 gap-y-0.5 text-[10px] leading-4">
              <span v-if="optionDescription(descriptor).defaultValue"><span class="font-medium text-foreground/70">默认</span> {{ optionDescription(descriptor).defaultValue }}</span>
              <span v-if="optionDescription(descriptor).range"><span class="font-medium text-foreground/70">范围</span> {{ optionDescription(descriptor).range }}</span>
              <span v-if="optionDescription(descriptor).step"><span class="font-medium text-foreground/70">步长</span> {{ optionDescription(descriptor).step }}</span>
              <span v-if="optionDescription(descriptor).choices.length > 0"><span class="font-medium text-foreground/70">可选</span> {{ optionDescription(descriptor).choices.join("、") }}</span>
            </div>
          </template>
          <input v-if="descriptor.option_type === 'check'" type="checkbox" :checked="optionChecked(descriptor)" class="size-4" @change="setOptionValue(descriptor, ($event.target as HTMLInputElement).checked)" />
          <Input v-else-if="descriptor.option_type === 'spin'" type="number" :min="descriptor.min ?? undefined" :max="descriptor.max ?? undefined" :step="descriptor.step ?? 1" :model-value="optionText(descriptor)" class="h-8 w-28 text-xs text-right" @change="setOptionValue(descriptor, ($event.target as HTMLInputElement).value)" />
          <Select v-else-if="descriptor.option_type === 'combo'" :model-value="optionText(descriptor)" @update:model-value="setOptionValue(descriptor, $event)"><SelectTrigger class="h-8 w-36 text-xs"><SelectValue /></SelectTrigger><SelectContent><SelectItem v-for="item in descriptor.vars" :key="item" :value="item">{{ item }}</SelectItem></SelectContent></Select>
          <div v-else-if="descriptor.option_type === 'string' || descriptor.option_type === 'file'" class="flex w-full min-w-0 items-center gap-1.5 sm:w-auto">
            <Input :model-value="optionText(descriptor)" :placeholder="isPathOption(descriptor) ? (descriptor.name.toLowerCase().includes('path') ? '未选择目录' : '未选择文件') : undefined" class="h-8 min-w-0 flex-1 text-xs" :class="isPathOption(descriptor) ? 'font-mono sm:w-48' : 'sm:w-48'" @change="setOptionValue(descriptor, ($event.target as HTMLInputElement).value)" />
            <Button v-if="isPathOption(descriptor)" variant="outline" size="sm" class="h-8 shrink-0 gap-1 text-xs" :title="descriptor.name.toLowerCase().includes('path') ? '选择目录' : '选择文件'" @click="pickOptionPath(descriptor)"><FolderOpen class="size-3.5" />选择</Button>
          </div>
          <Button v-else variant="outline" size="sm" class="h-8 text-xs" :disabled="!engine.running || engine.analyzing" @click="triggerOption(descriptor)">触发</Button>
        </SettingRow>
      </div>
    </div>

    <Separator class="my-0.5" /><h4 class="text-xs font-semibold text-foreground">默认分析策略</h4>
    <div class="flex flex-col gap-2.5 rounded-lg border bg-card/60 p-3"><SettingRow label="思考控制模式" description="发起思考分析时的限制策略"><Select v-model="analysis.mode" @update:model-value="saveProfileAndSync"><SelectTrigger class="h-8 w-full text-xs sm:w-40"><SelectValue /></SelectTrigger><SelectContent><SelectGroup><SelectItem value="fixed_time">固定用时 (毫秒)</SelectItem><SelectItem value="fixed_depth">固定搜索深度</SelectItem><SelectItem value="fixed_nodes">固定节点总数</SelectItem><SelectItem value="infinite">无限思考模式</SelectItem></SelectGroup></SelectContent></Select></SettingRow><SettingRow v-if="analysis.mode !== 'infinite'" label="限制数值" :description="analysis.mode === 'fixed_time' ? '单步最大思考时间 (ms)' : analysis.mode === 'fixed_depth' ? '最大搜索层深 (步)' : '最大搜索节点数'"><Input v-model.number="analysis.value" type="number" min="1" class="h-8 w-full text-right font-mono sm:w-28" @change="saveProfileAndSync" /></SettingRow><SettingRow label="候选分支数 (Multi-PV)" description="同时计算并展示前 N 条最佳候选着法"><Select :model-value="String(analysis.multiPv)" @update:model-value="analysis.multiPv = Number($event); saveProfileAndSync()"><SelectTrigger class="h-8 w-full text-xs sm:w-28"><SelectValue /></SelectTrigger><SelectContent><SelectGroup><SelectItem value="1">1 条最优分支</SelectItem><SelectItem value="2">2 条候选分支</SelectItem><SelectItem value="3">3 条候选分支</SelectItem><SelectItem value="5">5 条候选分支</SelectItem></SelectGroup></SelectContent></Select></SettingRow></div>
    <div class="flex justify-end gap-2 pt-1"><Button size="sm" class="h-8 text-xs" @click="applyEngineAndRestart">保存并应用当前引擎</Button></div>
  </div>
</template>
