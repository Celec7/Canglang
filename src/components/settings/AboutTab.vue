<script setup lang="ts">
import { computed, ref } from "vue";
import {
  Badge,
  Button,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui";
import {
  Check,
  Copy,
  Cpu,
  ExternalLink,
  Heart,
  ShieldCheck,
} from "@lucide/vue";
import { useToast } from "@/composables/useToast";
import { usePreferencesStore } from "@/stores/preferences";
import { openUrl } from "@tauri-apps/plugin-opener";
import logoUrl from "@/assets/logo.svg";

const REPOSITORY_URL = "https://github.com/Celec7/Canglang";
const ACKNOWLEDGEMENT_URLS = {
  xqbase: "https://github.com/xqbase/eleeye",
  chessDb: "https://github.com/noobpwnftw/chessdb",
  pikafish: "https://github.com/official-pikafish/Pikafish",
  tauri: "https://github.com/tauri-apps/tauri",
  rust: "https://github.com/rust-lang/rust",
  vue: "https://github.com/vuejs/core",
} as const;

const preferences = usePreferencesStore();
const { show } = useToast();
const copied = ref(false);
const activeSubTab = ref("environment");

const systemInfo = computed(() => {
  const isBrowser = typeof window !== "undefined";
  const platform = isBrowser ? navigator.platform || "Unknown" : "Unknown";
  const userAgent = isBrowser ? navigator.userAgent : "Unknown";
  const screenRes = isBrowser ? `${window.screen.width} × ${window.screen.height}` : "Unknown";
  return {
    appVersion: typeof __APP_VERSION__ !== "undefined" ? `v${__APP_VERSION__}` : "v0.1.0",
    rustEdition: "2024",
    tauriVersion: "v2.11",
    frontendStack: "Vue 3.5 · Vite 8 · Pinia · Tailwind 4",
    uiSystem: "Radix-Vue · Shadcn Architecture",
    ruleProfiles: "国标 2020 (china2020) · 亚规 2017 (asian2017)",
    bookFormat: "本地 .bh (SQLite) + 象棋云库 (chessdb.cn 在线协同)",
    platform,
    screenRes,
    userAgent,
    theme: preferences.theme === "dark" ? "深色模式 (Dark)" : "浅色模式 (Light)",
  };
});

async function copyDiagnostics() {
  const info = systemInfo.value;
  const diagnosticText = `### Canglang 沧浪象棋 诊断信息
- 版本: ${info.appVersion}
- 核心: Rust ${info.rustEdition} · Tauri ${info.tauriVersion}
- 前端: ${info.frontendStack}
- 规则档案: ${info.ruleProfiles}
- 开局库: ${info.bookFormat}
- 配置存储: ${preferences.locationInfo?.isPortable ? "便携模式" : "系统配置目录"} (\`${preferences.locationInfo?.filePath || "config.json"}\`)
- 平台: ${info.platform}
- 分辨率: ${info.screenRes}
- 主题: ${info.theme}
- User Agent: \`${info.userAgent}\`
`;

  try {
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(diagnosticText);
      copied.value = true;
      show("诊断信息已复制到剪贴板，可直接粘贴用于 Issue 反馈");
      setTimeout(() => {
        copied.value = false;
      }, 2500);
    }
  } catch {
    show("复制到剪贴板失败，请手动选取");
  }
}

async function openExternalUrl(url: string) {
  try {
    await openUrl(url);
  } catch {
    show(`无法调用系统浏览器，请手动访问 ${url}`);
  }
}

async function openRepository() {
  await openExternalUrl(REPOSITORY_URL);
}
</script>

<template>
  <div class="flex flex-col gap-4 text-xs">
    <!-- 顶部品牌标牌 -->
    <div class="flex items-start justify-between gap-4 rounded-xl border bg-gradient-to-br from-card to-muted/30 p-4 shadow-xs">
      <div class="flex items-center gap-3.5">
        <div class="relative flex size-12 shrink-0 items-center justify-center rounded-xl border bg-card p-1 shadow-xs">
          <img :src="logoUrl" alt="沧浪象棋 Logo" class="size-full object-contain" />
        </div>
        <div class="space-y-1">
          <div class="flex items-center gap-2">
            <h3 class="text-base font-bold tracking-tight text-foreground font-sans">
              Canglang <span class="text-xs font-normal text-muted-foreground font-sans">沧浪象棋</span>
            </h3>
            <Badge variant="secondary" class="h-4.5 px-1.5 text-[10px] font-mono font-normal">
              {{ systemInfo.appVersion }}
            </Badge>
          </div>
          <p class="text-[11px] text-muted-foreground leading-snug">
            中国象棋桌面应用
          </p>
        </div>
      </div>

      <!-- 快速操作按钮组 -->
      <div class="flex items-center gap-1.5 shrink-0">
        <Button
          variant="outline"
          size="sm"
          class="h-7 gap-1 px-2.5 text-[11px]"
          title="复制版本与系统诊断信息用于 Issue 反馈"
          @click="copyDiagnostics"
        >
          <Check v-if="copied" class="size-3 text-primary" />
          <Copy v-else class="size-3 text-muted-foreground" />
          <span>{{ copied ? "已复制" : "复制诊断" }}</span>
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-7 gap-1 px-2 text-[11px] text-muted-foreground hover:text-foreground"
          title="访问开源代码仓库"
          @click="openRepository"
        >
          <ExternalLink class="size-3" />
          <span>开源仓库</span>
        </Button>
      </div>
    </div>

    <!-- Qt / KDE 风格分栏子页签 -->
    <Tabs v-model="activeSubTab" class="flex flex-col gap-3">
      <TabsList class="h-8 w-full justify-start rounded-lg bg-muted/40 p-0.5">
        <TabsTrigger value="environment" class="h-7 gap-1.5 px-3 text-xs font-medium data-[state=active]:shadow-xs">
          <Cpu class="size-3" />
          <span>环境</span>
        </TabsTrigger>
        <TabsTrigger value="acknowledgments" class="h-7 gap-1.5 px-3 text-xs font-medium data-[state=active]:shadow-xs">
          <Heart class="size-3" />
          <span>鸣谢</span>
        </TabsTrigger>
        <TabsTrigger value="license" class="h-7 gap-1.5 px-3 text-xs font-medium data-[state=active]:shadow-xs">
          <ShieldCheck class="size-3" />
          <span>许可证</span>
        </TabsTrigger>
      </TabsList>

      <!-- 1. 环境 -->
      <TabsContent value="environment" class="mt-0">
        <div class="rounded-lg border divide-y bg-card text-xs">
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">应用版本</span>
            <span class="font-mono font-medium text-foreground">{{ systemInfo.appVersion }}</span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">核心</span>
            <span class="font-mono text-foreground">Rust {{ systemInfo.rustEdition }} · canglang_app</span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">桌面运行时</span>
            <span class="font-mono text-foreground">Tauri {{ systemInfo.tauriVersion }} · tauri-specta</span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">前端</span>
            <span class="font-mono text-foreground">{{ systemInfo.frontendStack }}</span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">UI 组件</span>
            <span class="text-foreground">{{ systemInfo.uiSystem }}</span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">规则档案</span>
            <span class="text-foreground">{{ systemInfo.ruleProfiles }}</span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">开局库</span>
            <span class="text-foreground">{{ systemInfo.bookFormat }}</span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">平台</span>
            <span class="font-mono text-foreground">{{ systemInfo.platform }}</span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">分辨率</span>
            <span class="font-mono text-foreground">{{ systemInfo.screenRes }}</span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">配置模式</span>
            <span
              class="inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] font-medium"
              :class="preferences.locationInfo?.isPortable ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400' : 'bg-muted text-muted-foreground'"
            >
              <span>{{ preferences.locationInfo?.isPortable ? "便携模式 (Portable)" : "系统配置目录" }}</span>
            </span>
          </div>
          <div class="flex items-center justify-between p-2.5">
            <span class="text-muted-foreground">配置路径</span>
            <span class="font-mono text-[11px] text-foreground truncate max-w-64" :title="preferences.locationInfo?.filePath">
              {{ preferences.locationInfo?.filePath || "config.json" }}
            </span>
          </div>
        </div>
      </TabsContent>

      <!-- 2. 鸣谢 -->
      <TabsContent value="acknowledgments" class="mt-0 space-y-2.5">
        <div class="rounded-lg border divide-y bg-card text-xs">
          <div class="p-2.5 space-y-0.5">
            <div class="flex items-center justify-between">
              <button type="button" class="font-semibold text-foreground hover:text-primary hover:underline" @click="openExternalUrl(ACKNOWLEDGEMENT_URLS.xqbase)">xqbase / ElephantEye</button>
              <Badge variant="outline" class="text-[10px] h-4">协议</Badge>
            </div>
            <p class="text-[11px] text-muted-foreground">UCCI 协议</p>
          </div>

          <div class="p-2.5 space-y-0.5">
            <div class="flex items-center justify-between">
              <button type="button" class="font-semibold text-foreground hover:text-primary hover:underline" @click="openExternalUrl(ACKNOWLEDGEMENT_URLS.chessDb)">象棋云库 (chessdb.cn)</button>
              <Badge variant="outline" class="text-[10px] h-4">云端数据</Badge>
            </div>
            <p class="text-[11px] text-muted-foreground">在线开局库</p>
          </div>

          <div class="p-2.5 space-y-0.5">
            <div class="flex items-center justify-between">
              <button type="button" class="font-semibold text-foreground hover:text-primary hover:underline" @click="openExternalUrl(ACKNOWLEDGEMENT_URLS.pikafish)">Pikafish</button>
              <Badge variant="outline" class="text-[10px] h-4">引擎</Badge>
            </div>
            <p class="text-[11px] text-muted-foreground">内置分析引擎</p>
          </div>

          <div class="p-2.5 space-y-0.5">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-1 font-semibold text-foreground">
                <button type="button" class="hover:text-primary hover:underline" @click="openExternalUrl(ACKNOWLEDGEMENT_URLS.tauri)">Tauri</button>
                <span>·</span>
                <button type="button" class="hover:text-primary hover:underline" @click="openExternalUrl(ACKNOWLEDGEMENT_URLS.rust)">Rust</button>
                <span>·</span>
                <button type="button" class="hover:text-primary hover:underline" @click="openExternalUrl(ACKNOWLEDGEMENT_URLS.vue)">Vue</button>
              </div>
              <Badge variant="outline" class="text-[10px] h-4">基础框架</Badge>
            </div>
            <p class="text-[11px] text-muted-foreground">桌面运行时、核心语言和前端框架</p>
          </div>
        </div>
      </TabsContent>

      <!-- 3. 许可证 -->
      <TabsContent value="license" class="mt-0 space-y-2.5">
        <div class="rounded-lg border bg-muted/20 p-3 space-y-2 font-mono text-[11px] leading-relaxed text-muted-foreground">
          <div class="text-foreground font-semibold">MIT License</div>
          <p>Copyright (c) 2026 Cccc_</p>
          <p>Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software.</p>
          <p>The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.</p>
          <p>THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.</p>
        </div>

      </TabsContent>
    </Tabs>
  </div>
</template>
