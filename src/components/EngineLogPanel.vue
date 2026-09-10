<script setup lang="ts">
import { computed, onScopeDispose, ref } from "vue";
import { Button } from "@/components/ui";
import { commands, listenEngineProtocol, unwrap, type UnlistenFn } from "@/lib/ipc";
import type { EngineRawLine } from "@/lib/events";
import { useEngineStore } from "@/stores/engine";

const open = ref(false);
const lines = ref<EngineRawLine[]>([]);
const engine = useEngineStore();
const streamFilter = ref<EngineRawLine["stream"] | "all">("all");

function formatTimestamp(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString();
}

const visibleLines = computed(() => streamFilter.value === "all"
  ? lines.value
  : lines.value.filter((line) => line.stream === streamFilter.value));
let unlisten: UnlistenFn | null = null;
let isSubscribing = false;

function stopListening() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}

function mergeLines(...batches: EngineRawLine[][]): EngineRawLine[] {
  const byKey = new Map<string, EngineRawLine>();
  for (const batch of batches) {
    for (const line of batch) {
      byKey.set(`${line.run_id}:${line.sequence}`, line);
    }
  }
  return [...byKey.values()].sort((a, b) => a.sequence - b.sequence);
}

async function startListening() {
  if (unlisten || isSubscribing) return;
  isSubscribing = true;
  try {
    const cleanup = await listenEngineProtocol((line) => {
      const currentRunId = lines.value[0]?.run_id;
      lines.value = currentRunId && currentRunId !== line.run_id
        ? [line]
        : mergeLines(lines.value, [line]).slice(-999);
    });

    if (!open.value) {
      cleanup();
      return;
    }

    unlisten = cleanup;

    const snapshot = await unwrap(await commands.engineProtocolLog(engine.lastAnalysisSessionId ?? null)).catch(() => []);
    if (!open.value) return;

    const currentRunId = lines.value[0]?.run_id;
    const snapshotRunId = snapshot[0]?.run_id;
    if (currentRunId && (!snapshotRunId || currentRunId !== snapshotRunId)) return;
    lines.value = mergeLines(snapshot, lines.value).slice(-999);
  } finally {
    isSubscribing = false;
  }
}

async function toggle() {
  open.value = !open.value;
  if (open.value) {
    await startListening();
  } else {
    stopListening();
  }
}

function clearDisplay() {
  lines.value = [];
}

async function exportLog() {
  const path = await unwrap(await commands.enginePickFile("protocol-log"));
  if (path) await unwrap(await commands.engineExportProtocolLog(path, engine.lastAnalysisSessionId ?? null));
}

async function copyVisibleLog() {
  await navigator.clipboard.writeText(visibleLines.value.map((line) => line.raw).join(""));
}

onScopeDispose(() => stopListening());

defineExpose({
  toggle,
  open,
  lines,
});
</script>

<template>
  <div class="rounded-lg border bg-card/60">
    <button type="button" class="flex w-full items-center justify-between px-3 py-2 text-xs font-medium" @click="toggle">
      <span>协议日志 <span v-if="lines.length" class="text-[10px] text-muted-foreground">({{ lines.length }})</span></span>
      <span class="text-[10px] text-muted-foreground">{{ open ? "收起" : "展开" }}</span>
    </button>
    <div v-if="open" class="border-t p-2">
      <div class="mb-2 flex gap-1">
        <Button variant="outline" size="sm" class="h-6 text-[10px]" @click="clearDisplay">清空显示</Button>
        <Button variant="outline" size="sm" class="h-6 text-[10px]" @click="copyVisibleLog">复制显示</Button>
        <Button variant="outline" size="sm" class="h-6 text-[10px]" @click="exportLog">导出当前日志</Button>
        <select v-model="streamFilter" class="h-6 rounded border bg-background px-1 text-[10px]" aria-label="协议日志流过滤">
          <option value="all">全部</option>
          <option value="stdin">发送</option>
          <option value="stdout">stdout</option>
          <option value="stderr">stderr</option>
          <option value="lifecycle">生命周期</option>
        </select>
      </div>
      <pre class="max-h-44 overflow-auto whitespace-pre-wrap break-all rounded bg-muted/40 p-2 font-mono text-[10px] leading-relaxed">{{ visibleLines.map((line) => `${formatTimestamp(line.timestamp_ms)} #${line.sequence} ${line.run_id} ${line.analysis_session_id ?? "preamble"} ${line.stream}: ${line.raw}`).join("") || "暂无协议日志" }}</pre>
    </div>
  </div>
</template>
