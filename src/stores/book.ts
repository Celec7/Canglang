import { defineStore } from "pinia";
import { ref } from "vue";
import type { BookMove, CloudBookMode, CloudBookStatus } from "@/bindings";
import { commands, unwrap } from "@/lib/ipc";
import { useGameStore } from "@/stores/game";

export const useBookStore = defineStore("book", () => {
  const loaded = ref<string[]>([]);
  const moves = ref<BookMove[]>([]);
  const loading = ref(false);
  let queryVersion = 0;
  const cloudStatus = ref<CloudBookStatus>({
    enabled: true,
    mode: "hybrid",
  });

  async function load(path: string) {
    await unwrap(await commands.bookLoad(path));
    loaded.value = await unwrap(await commands.bookLoaded());
  }

  async function unload(path: string) {
    await unwrap(await commands.bookUnload(path));
    loaded.value = await unwrap(await commands.bookLoaded());
  }

  async function clear() {
    await unwrap(await commands.bookClear());
    loaded.value = [];
  }

  async function pickFile(): Promise<string | null> {
    return unwrap(await commands.enginePickFile("book"));
  }

  async function query(fen: string) {
    if (!useGameStore().capabilities.query_book.enabled) {
      clearQuery();
      return;
    }
    const version = ++queryVersion;
    moves.value = [];
    loading.value = true;
    try {
      const result = await unwrap(await commands.bookQuery(fen));
      if (version === queryVersion) moves.value = result;
    } finally {
      if (version === queryVersion) loading.value = false;
    }
  }

  function clearQuery() {
    // 关闭查询来源后，迟到的响应也不能重新填充候选列表
    queryVersion++;
    moves.value = [];
    loading.value = false;
  }

  async function setCloudEnabled(enabled: boolean) {
    await unwrap(await commands.bookSetCloudEnabled(enabled));
    cloudStatus.value.enabled = enabled;
  }

  async function setCloudMode(mode: CloudBookMode) {
    await unwrap(await commands.bookSetCloudMode(mode));
    cloudStatus.value.mode = mode;
  }

  async function refreshCloudStatus() {
    try {
      cloudStatus.value = await unwrap(await commands.bookGetCloudStatus());
    } catch {
      // 容错降级
    }
  }

  return {
    loaded,
    moves,
    loading,
    cloudStatus,
    load,
    unload,
    clear,
    pickFile,
    query,
    clearQuery,
    setCloudEnabled,
    setCloudMode,
    refreshCloudStatus,
  };
});
