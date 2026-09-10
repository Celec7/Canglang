<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  Badge,
  Button,
  Input,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui";
import {
  AlignJustify,
  Cloud,
  CloudOff,
  Folder,
  LayoutList,
  Loader2,
  Plus,
} from "@lucide/vue";
import { parseBookEvaluation } from "@/lib/book-evaluation";
import { useBookStore } from "@/stores/book";
import { useGameStore } from "@/stores/game";
import { usePreferencesStore } from "@/stores/preferences";

const book = useBookStore();
const game = useGameStore();
const preferences = usePreferencesStore();
const bookPath = ref("");
const showPathInput = ref(false);
const viewMode = ref<"cards" | "compact">("cards");

async function loadBook() {
  const target = bookPath.value.trim();
  if (!target) return;
  await book.load(target);
  preferences.addOpeningBookPath(target);
  showPathInput.value = false;
  bookPath.value = "";
  if (game.capabilities.query_book.enabled && game.fen) await book.query(game.fen);
}

async function playMove(iccs: string) {
  await game.makeMove(iccs);
}

const canQueryBook = computed(() => {
  return (
    book.loaded.length > 0 ||
    (preferences.cloudBookEnabled && preferences.cloudBookMode !== "local_only")
  );
});

// 当开局库配置或局面变化时，自动执行本地/云端检索
watch(
  () => [game.fen, preferences.cloudBookEnabled, preferences.cloudBookMode, book.loaded.length] as const,
  ([fen, cloudEnabled, cloudMode, loadedCount]) => {
    const active = loadedCount > 0 || (cloudEnabled && cloudMode !== "local_only");
    if (active && fen && game.capabilities.query_book.enabled) {
      void book.query(fen);
    } else {
      book.clearQuery();
    }
  },
  { immediate: true }
);

function isCloudSource(source: string): boolean {
  return source.includes("云库");
}

</script>

<template>
  <div class="flex flex-col flex-1 min-h-0 h-full gap-2">
    <!-- 控制栏：来源概览与视图切换 -->
    <div class="flex items-center justify-between text-xs px-0.5">
      <div class="flex items-center gap-1.5 min-w-0">
        <span class="font-medium text-foreground text-xs">
          候选招法
          <span v-if="book.moves.length > 0" class="text-muted-foreground font-mono font-normal">
            ({{ book.moves.length }})
          </span>
        </span>

        <!-- 来源状态指示徽标 -->
        <Badge
          variant="outline"
          class="h-4 px-1 text-[10px] gap-1 font-normal"
          :class="preferences.cloudBookEnabled && preferences.cloudBookMode !== 'local_only' ? 'text-sky-600 dark:text-sky-400 border-sky-500/30' : 'text-muted-foreground'"
        >
          <Cloud v-if="preferences.cloudBookEnabled && preferences.cloudBookMode !== 'local_only'" class="size-2.5" />
          <Folder v-else class="size-2.5" />
          <span>{{ preferences.cloudBookEnabled && preferences.cloudBookMode !== 'local_only' ? '云库在线' : '本地' }}</span>
        </Badge>
      </div>

      <div class="flex items-center gap-1 shrink-0">
        <Button
          variant="ghost"
          size="icon"
          class="size-5.5 rounded"
          :title="viewMode === 'cards' ? '切换为紧凑单行视图' : '切换为卡片自适应视图'"
          @click="viewMode = viewMode === 'cards' ? 'compact' : 'cards'"
        >
          <LayoutList v-if="viewMode === 'cards'" class="size-3" />
          <AlignJustify v-else class="size-3" />
        </Button>

        <Button
          variant="ghost"
          size="icon"
          class="size-5.5 rounded"
          title="添加本地 .bh 开局库文件"
          @click="showPathInput = !showPathInput"
        >
          <Plus class="size-3" />
        </Button>
      </div>
    </div>

    <!-- 本地路径配置栏（默认收起，按需展开） -->
    <div v-if="showPathInput" class="flex gap-1.5 p-2 rounded-md bg-muted/40 border text-xs">
      <Input
        v-model="bookPath"
        placeholder="输入本地 .bh 开局库绝对路径"
        class="h-7 text-xs flex-1"
        @keyup.enter="loadBook"
      />
      <Button size="sm" class="h-7 px-2 text-xs" @click="loadBook">加载</Button>
      <Button variant="ghost" size="sm" class="h-7 px-2 text-xs" @click="showPathInput = false">取消</Button>
    </div>

    <!-- 1. 自适应卡片视图模式：适合 200px~360px 侧边栏并撑满剩余空间 -->
    <div
      v-if="book.moves.length > 0 && viewMode === 'cards'"
      class="flex-1 min-h-0 overflow-y-auto space-y-1.5 pr-0.5"
    >
      <div
        v-for="(m, i) in book.moves"
        :key="i"
        class="group flex flex-col gap-1 rounded-lg border border-border/60 bg-card p-2 text-xs transition-all hover:border-primary/40 hover:bg-accent/40 cursor-pointer shadow-2xs"
        title="点击在棋盘试走此招"
        @click="playMove(m.iccs)"
      >
        <!-- 主信息行：评价徽标 + 中文走法 + ICCS + 评分 + 胜率 + 来源 -->
        <div class="flex items-center justify-between gap-1.5 min-w-0">
          <div class="flex items-center gap-1.5 min-w-0">
            <!-- 评价状态徽标 -->
            <span
              class="inline-flex items-center justify-center rounded px-1.5 py-0.5 text-[10px] border shadow-2xs shrink-0 font-medium"
              :class="parseBookEvaluation(m.note).badgeClass"
            >
              {{ parseBookEvaluation(m.note).label }}
            </span>

            <!-- 中文走法大字 -->
            <span class="font-bold text-foreground tracking-wide text-xs truncate">
              {{ m.notation || m.iccs }}
            </span>
          </div>

          <!-- 右侧：胜率与分值 -->
          <div class="flex items-center gap-1.5 shrink-0">
            <span
              class="font-mono text-[11px] tabular-nums"
              :class="m.score > 0 ? 'text-primary font-semibold' : m.score < 0 ? 'text-muted-foreground' : 'text-muted-foreground/70'"
            >
              {{ m.score > 0 ? `+${m.score}` : m.score }}
            </span>
            <span class="font-semibold text-primary text-xs tabular-nums">
              {{ (m.win_rate ?? 0).toFixed(1) }}%
            </span>

            <!-- 来源标记 -->
            <span
              class="inline-flex items-center gap-0.5 rounded px-1 py-0.5 text-[10px] shrink-0"
              :class="isCloudSource(m.source) ? 'bg-sky-500/10 text-sky-600 dark:text-sky-400' : 'bg-muted text-muted-foreground'"
              :title="m.source"
            >
              <Cloud v-if="isCloudSource(m.source)" class="size-2.5" />
              <Folder v-else class="size-2.5" />
              <span class="hidden sm:inline">{{ isCloudSource(m.source) ? (m.source.includes('残局') ? '残局' : '云') : '本' }}</span>
            </span>
          </div>
        </div>

        <!-- 副信息行：变例分支详情与迷你胜率微柱 -->
        <div class="flex items-center justify-between text-[10px] text-muted-foreground pt-1 border-t border-border/40 gap-2">
          <Tooltip>
            <TooltipTrigger as-child>
              <span class="truncate cursor-help">
                {{ parseBookEvaluation(m.note).branchesText || (m.note ? m.note : "标准开局推荐") }}
              </span>
            </TooltipTrigger>
            <TooltipContent class="text-xs max-w-xs leading-relaxed">
              {{ parseBookEvaluation(m.note).tooltip }}
            </TooltipContent>
          </Tooltip>

          <!-- 战绩或迷你胜率条 -->
          <span v-if="m.win_count || m.draw_count || m.lose_count" class="font-mono tabular-nums shrink-0">
            {{ m.win_count }}胜 / {{ m.draw_count }}和 / {{ m.lose_count }}负
          </span>
          <div v-else class="w-14 h-1 rounded-full bg-muted overflow-hidden shrink-0">
            <div
              class="h-full bg-primary/70 rounded-full"
              :style="{ width: `${Math.min(100, Math.max(0, m.win_rate ?? 50))}%` }"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- 2. 极简紧凑单行模式：撑满剩余空间 -->
    <div
      v-else-if="book.moves.length > 0 && viewMode === 'compact'"
      class="flex-1 min-h-0 overflow-y-auto rounded-lg border divide-y bg-card text-xs"
    >
      <div
        v-for="(m, i) in book.moves"
        :key="i"
        class="flex items-center justify-between px-2.5 py-1.5 transition-colors hover:bg-accent/50 cursor-pointer gap-2"
        title="点击在棋盘试走此招"
        @click="playMove(m.iccs)"
      >
        <div class="flex items-center gap-1.5 min-w-0">
          <span
            class="size-1.5 rounded-full shrink-0"
            :class="m.score > 0 ? 'bg-primary' : m.score < 0 ? 'bg-amber-500' : 'bg-muted-foreground'"
          />
          <span class="font-bold text-foreground text-xs truncate">
            {{ m.notation || m.iccs }}
          </span>
          <span
            class="text-[10px] px-1 py-0.2 rounded border shrink-0"
            :class="parseBookEvaluation(m.note).badgeClass"
          >
            {{ parseBookEvaluation(m.note).label }}
          </span>
        </div>

        <div class="flex items-center gap-2 shrink-0 font-mono text-[11px]">
          <span class="tabular-nums" :class="m.score > 0 ? 'text-primary font-medium' : 'text-muted-foreground'">
            {{ m.score > 0 ? `+${m.score}` : m.score }}
          </span>
          <span class="font-semibold text-primary tabular-nums">
            {{ (m.win_rate ?? 0).toFixed(1) }}%
          </span>
        </div>
      </div>
    </div>

    <!-- 加载中状态 -->
    <div
      v-else-if="book.loading"
      class="flex items-center justify-center gap-2 rounded-lg border border-dashed px-3 py-4 text-xs text-muted-foreground bg-muted/20 animate-pulse"
    >
      <Loader2 class="size-3.5 animate-spin text-primary" />
      <span>正在检索开局库 (本地 / 云端)...</span>
    </div>

    <!-- 脱谱或未加载状态 -->
    <div v-else class="flex flex-col gap-1.5 rounded-lg border border-dashed px-3 py-3 text-xs text-muted-foreground bg-muted/20">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-1.5 min-w-0">
          <Cloud v-if="preferences.cloudBookEnabled && preferences.cloudBookMode !== 'local_only'" class="size-3 text-sky-500 shrink-0" />
          <CloudOff v-else class="size-3 text-muted-foreground/60 shrink-0" />
          <span class="truncate">
            {{
              !canQueryBook
                ? "未配置开局库（本地库与云库均未开启）"
                : "当前局面已脱谱（无匹配开局走法）"
            }}
          </span>
        </div>
      </div>

      <div class="flex items-center justify-between pt-1 border-t border-border/30 text-[11px]">
        <span>{{ book.loaded.length > 0 ? `已挂载 ${book.loaded.length} 个本地库` : "尚未载入本地库" }}</span>
        <button
          type="button"
          class="text-primary hover:underline"
          @click="showPathInput = !showPathInput"
        >
          + 载入 .bh 库
        </button>
      </div>
    </div>
  </div>
</template>
