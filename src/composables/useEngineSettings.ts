import { computed, onScopeDispose, reactive, ref, watch, type Ref } from "vue";
import type { EngineOptionDescriptor, EngineOptionValue, EngineProfile } from "@/bindings";
import { engineConfigFromProfile } from "@/lib/engine-profile";
import type { AnalysisModeType } from "@/lib/preferences";
import type { DownloadProgressPayload } from "@/lib/events";
import { getDefaultHardwareDefaults } from "@/lib/preferences";
import { commands, listenDownloadProgress, unwrap, type UnlistenFn } from "@/lib/ipc";
import { useEngineStore } from "@/stores/engine";
import { usePreferencesStore } from "@/stores/preferences";
import { useToast } from "@/composables/useToast";

const BUILTIN_PROFILE: EngineProfile = {
  id: "builtin-pikafish",
  name: "Pikafish",
  path: "",
  protocol: "uci",
  threads: null,
  hashMb: null,
  nnuePath: null,
  optionOverrides: {},
  releaseRevision: "2026-09-06",
  installedRevision: "",
  isBuiltin: true,
  downloadUrl:
    "https://github.com/official-pikafish/Pikafish/releases/download/Pikafish-2026-09-06/Pikafish.2026-09-06.7z",
  description: "现代顶尖的中国象棋开源 NNUE 深度神经网络引擎，支持深度多路分析与残局推算。",
};

const ENGINE_OPTION_HELP: Record<string, string> = {
  threads: "参与搜索的 CPU 线程数；越大通常越快，也会占用更多处理器资源。",
  hash: "置换表容量（MB）；用于缓存已搜索局面，过大可能挤占系统内存。",
  multipv: "同时保留并展示的候选主变数量；数值越大，每条主变通常会得到更少的搜索资源。",
  ponder: "是否在对手思考时继续预判；只对支持 ponder 的对弈模式生效。",
  evalfile: "主 NNUE 评价网络文件；更换后通常需要重新应用引擎才能生效。",
  evalfilesmall: "轻量 NNUE 评价网络文件；由引擎在浅层搜索阶段使用。",
  usennue: "是否启用 NNUE 评价；关闭后会退回传统评价函数，棋力和速度可能变化。",
  syzygypath: "残局库目录；仅在引擎声明支持对应残局库格式时生效。",
  nalimovpath: "Nalimov 残局库目录；仅在引擎支持该格式时生效。",
  bookfile: "引擎内部开局库文件；这里只影响引擎自身，不会覆盖 Canglang 的辅助开局库。",
  bookpath: "引擎内部开局库目录；这里只影响引擎自身，不会覆盖 Canglang 的辅助开局库。",
  usebook: "是否启用引擎内部开局库；不会改变 Canglang 的独立开局库展示。",
  moveoverhead: "为通信和界面响应预留的时间余量（毫秒）；对限时对弈尤其重要。",
  slowmover: "调整引擎在时限下的用时倾向；值越大通常越愿意思考更久。",
  ucilimitstrength: "是否限制引擎强度；开启后通常需要配合 UCI_Elo 使用。",
  ucielo: "目标等级；只有引擎开启强度限制时才生效。",
  ucishowwdl: "是否输出胜和负概率，供分析界面辅助展示。",
  clearhash: "清空引擎已缓存的置换表，不修改棋局或当前 Profile。",
};

const ENGINE_OPTION_TYPE_HELP: Record<EngineOptionDescriptor["option_type"], string> = {
  check: "布尔开关",
  spin: "整数参数",
  combo: "枚举参数",
  string: "文本参数",
  file: "文件路径",
  button: "即时操作",
};

type EngineOptionDescription = {
  summary: string;
  defaultValue: string | null;
  range: string | null;
  step: string | null;
  choices: string[];
};

export function useEngineSettings(open: Ref<boolean>) {
  const hardware = getDefaultHardwareDefaults();
  const engine = useEngineStore();
  const preferences = usePreferencesStore();
  const { show } = useToast();
  const selectedProfileId = ref("");
  const showAdvanced = ref(false);
  const isDownloading = ref(false);
  const downloadProgress = ref<DownloadProgressPayload | null>(null);
  const downloadUnlisten = ref<UnlistenFn | null>(null);

  const currentProfile = reactive<EngineProfile>({
    ...BUILTIN_PROFILE,
    threads: hardware.threads,
    hashMb: hardware.hashMb,
  });
  const analysis = reactive({
    mode: preferences.analysisMode as AnalysisModeType,
    value: preferences.analysisLimitValue,
    multiPv: preferences.multiPv,
  });

  const builtinNotInstalled = computed(
    () => currentProfile.isBuiltin && !currentProfile.path.trim()
  );
  const builtinNeedsUpgrade = computed(
    () =>
      currentProfile.isBuiltin &&
      !!currentProfile.path.trim() &&
      !!currentProfile.releaseRevision &&
      currentProfile.installedRevision !== currentProfile.releaseRevision,
  );
  const optionDescriptors = computed(() =>
    currentProfile.id === preferences.activeEngineId
      ? (engine.engineInfo?.option_descriptors ?? []).filter((descriptor) =>
        !["threads", "hash", "multipv"].includes(normalizeOptionName(descriptor.name)),
      )
      : [],
  );
  const activeDownload = computed(() =>
    downloadProgress.value?.profileId === currentProfile.id ? downloadProgress.value : null
  );
  const progressPercent = computed(() => {
    const percent = activeDownload.value?.percent ?? 0;
    return Number.isFinite(percent) ? Math.max(0, Math.min(100, percent)) : 0;
  });
  const downloadStageLabel = computed(() => {
    switch (activeDownload.value?.stage) {
      case "extracting":
        return "正在解压并配置可执行权限...";
      case "ready":
        return "装配完成";
      case "error":
        return "安装失败";
      default:
        return activeDownload.value?.message || "正在下载引擎压缩包...";
    }
  });

  function formatBytes(bytes: number): string {
    if (bytes <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB"];
    const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    return `${(bytes / Math.pow(1024, index)).toFixed(1)} ${units[index]}`;
  }

  function formatSpeed(bytesPerSecond: number): string {
    return `${formatBytes(bytesPerSecond)}/s`;
  }

  function loadProfileToForm(id: string) {
    const profile = preferences.engineProfiles.find((item) => item.id === id) ?? preferences.engineProfiles[0];
    if (!profile) return;
    Object.assign(currentProfile, {
      id: profile.id,
      name: profile.name,
      path: profile.path,
      protocol: profile.protocol,
      threads: profile.threads ?? hardware.threads,
      hashMb: profile.hashMb ?? hardware.hashMb,
      nnuePath: profile.nnuePath ?? null,
      isBuiltin: profile.isBuiltin ?? false,
      downloadUrl: profile.downloadUrl ?? null,
      description: profile.description ?? null,
      optionOverrides: profile.optionOverrides ?? {},
      releaseRevision: profile.releaseRevision ?? "",
      installedRevision: profile.installedRevision ?? "",
    });
  }

  function initEngineProfiles() {
    let profiles = preferences.engineProfiles;
    if (!profiles || profiles.length === 0) {
      const builtin = { ...BUILTIN_PROFILE, threads: hardware.threads, hashMb: hardware.hashMb };
      preferences.setEngineProfiles([builtin]);
      preferences.setActiveEngineId(builtin.id);
      profiles = [builtin];
    } else if (!profiles.some((profile) => profile.id === BUILTIN_PROFILE.id)) {
      profiles.unshift({ ...BUILTIN_PROFILE, threads: hardware.threads, hashMb: hardware.hashMb });
      preferences.setEngineProfiles(profiles);
    }

    const activeId = preferences.activeEngineId || profiles[0].id;
    selectedProfileId.value = activeId;
    loadProfileToForm(activeId);
    analysis.mode = preferences.analysisMode;
    analysis.value = preferences.analysisLimitValue;
    analysis.multiPv = preferences.multiPv;
  }

  watch(open, (isOpen) => {
    if (isOpen) initEngineProfiles();
  }, { immediate: true });

  function onSelectProfile(id: string) {
    if (id === "__new__") {
      const newId = `engine-${Date.now()}`;
      const newProfile: EngineProfile = {
        id: newId,
        name: `本地引擎 ${preferences.engineProfiles.filter((profile) => !profile.isBuiltin).length + 1}`,
        path: "",
        protocol: "auto",
        threads: hardware.threads,
        hashMb: hardware.hashMb,
        nnuePath: null,
        optionOverrides: {},
        releaseRevision: "",
        installedRevision: "",
        isBuiltin: false,
        downloadUrl: null,
        description: null,
      };
      preferences.saveEngineProfile(newProfile, { activate: false });
      selectedProfileId.value = newId;
      loadProfileToForm(newId);
      show("已新建本地引擎配置");
      return;
    }
    selectedProfileId.value = id;
    loadProfileToForm(id);
  }

  async function resetBuiltinEngine() {
    if (preferences.activeEngineId === currentProfile.id && engine.running) {
      try {
        await engine.stop();
      } catch (cause) {
        show(`停止当前引擎失败，未清除内置引擎: ${cause instanceof Error ? cause.message : String(cause)}`);
        return;
      }
    }
    try {
      const updated = await unwrap(await commands.engineRemoveBuiltin(currentProfile.id));
      preferences.saveEngineProfile(updated, { activate: false });
      loadProfileToForm(updated.id);
      show("已清除内置引擎的本地文件，可重新一键获取");
    } catch (cause) {
      show(`清除内置引擎失败: ${cause instanceof Error ? cause.message : String(cause)}`);
    }
  }

  async function deleteCurrentProfile() {
    if (currentProfile.isBuiltin) {
      await resetBuiltinEngine();
      return;
    }
    if (preferences.engineProfiles.length <= 1) {
      show("至少保留一个引擎配置");
      return;
    }
    if (preferences.activeEngineId === currentProfile.id && engine.running) {
      try {
        await engine.stop();
      } catch (cause) {
        show(`停止当前引擎失败，未删除配置: ${cause instanceof Error ? cause.message : String(cause)}`);
        return;
      }
    }
    preferences.removeEngineProfile(currentProfile.id);
    const next = preferences.engineProfiles[0];
    selectedProfileId.value = next.id;
    loadProfileToForm(next.id);
    show("引擎配置已删除");
  }

  function saveProfileAndSync() {
    preferences.saveEngineProfile({ ...currentProfile }, { activate: false });
    preferences.setAnalysisSettings(analysis.mode, analysis.value, analysis.multiPv);
  }

  async function handlePickEngineFile() {
    try {
      const result = await unwrap(await commands.enginePickFile("engine"));
      if (!result) return;
      currentProfile.path = result;
      if (currentProfile.name.startsWith("本地引擎")) {
        const basename = result.split(/[/\\]/).pop();
        if (basename) currentProfile.name = basename.replace(/\.[^/.]+$/, "");
      }
      saveProfileAndSync();
    } catch (cause) {
      show(`选择文件失败: ${cause instanceof Error ? cause.message : String(cause)}`);
    }
  }

  async function handlePickNnueFile() {
    try {
      const result = await unwrap(await commands.enginePickFile("nnue"));
      if (!result) return;
      currentProfile.nnuePath = result;
      saveProfileAndSync();
      show("已挂载外置 NNUE 权重，将在引擎启动时覆盖自带权重");
    } catch (cause) {
      show(`选择权重文件失败: ${cause instanceof Error ? cause.message : String(cause)}`);
    }
  }

  function clearNnuePath() {
    currentProfile.nnuePath = null;
    saveProfileAndSync();
    show("已恢复使用引擎自带的默认权重");
  }

  function optionValue(descriptor: EngineOptionDescriptor): EngineOptionValue | null {
    return currentProfile.optionOverrides?.[descriptor.name] ?? descriptor.default;
  }

  function normalizeOptionName(name: string): string {
    return name.replace(/[\s_-]/g, "").toLowerCase();
  }

  function optionValueText(value: EngineOptionValue | null | undefined): string {
    if (!value) return "";
    if (value.kind === "float" && value.value === null) return "未声明";
    return String(value.value);
  }

  function isPathOption(descriptor: EngineOptionDescriptor): boolean {
    if (descriptor.option_type === "file") return true;
    if (descriptor.option_type !== "string") return false;
    const normalized = normalizeOptionName(descriptor.name);
    return normalized.endsWith("path") || normalized.endsWith("file");
  }

  function optionPathKind(descriptor: EngineOptionDescriptor): "file" | "folder" {
    const normalized = normalizeOptionName(descriptor.name);
    return normalized.endsWith("path") ? "folder" : "file";
  }

  function optionDescription(descriptor: EngineOptionDescriptor): EngineOptionDescription {
    const description: EngineOptionDescription = {
      summary: ENGINE_OPTION_HELP[normalizeOptionName(descriptor.name)] ?? `由引擎声明的${ENGINE_OPTION_TYPE_HELP[descriptor.option_type]}。`,
      defaultValue: null,
      range: null,
      step: null,
      choices: [],
    };
    const defaultText = optionValueText(descriptor.default);
    if (defaultText) {
      description.defaultValue = isPathOption(descriptor) && defaultText === "<empty>" ? "未设置" : defaultText;
    }
    if (descriptor.option_type === "spin" && (descriptor.min !== null || descriptor.max !== null)) {
      description.range = `${descriptor.min ?? "不限"}～${descriptor.max ?? "不限"}`;
    }
    if (descriptor.step !== null) description.step = String(descriptor.step);
    if (descriptor.option_type === "combo" && descriptor.vars.length > 0) {
      description.choices = descriptor.vars;
    }
    return description;
  }

  function optionText(descriptor: EngineOptionDescriptor): string {
    const value = optionValue(descriptor);
    if (!value || value.kind === "bool") return "";
    const text = String(value.value);
    return isPathOption(descriptor) && text === "<empty>" ? "" : text;
  }

  function optionChecked(descriptor: EngineOptionDescriptor): boolean {
    return optionValue(descriptor)?.kind === "bool" && optionValue(descriptor)?.value === true;
  }

  function setOptionValue(descriptor: EngineOptionDescriptor, raw: string | boolean) {
    const next = { ...(currentProfile.optionOverrides ?? {}) };
    switch (descriptor.option_type) {
      case "check":
        next[descriptor.name] = { kind: "bool", value: Boolean(raw) };
        break;
      case "spin":
        next[descriptor.name] = { kind: "integer", value: Number(raw) };
        break;
      case "combo":
        next[descriptor.name] = { kind: "enum", value: String(raw) };
        break;
      case "file":
        next[descriptor.name] = { kind: "path", value: String(raw) };
        break;
      case "string":
        next[descriptor.name] = { kind: "string", value: String(raw) };
        break;
      case "button":
        return;
    }
    currentProfile.optionOverrides = next;
    saveProfileAndSync();
  }

  async function pickOptionPath(descriptor: EngineOptionDescriptor) {
    if (!isPathOption(descriptor)) return;
    try {
      const result = await unwrap(await commands.enginePickFile(optionPathKind(descriptor)));
      if (result) setOptionValue(descriptor, result);
    } catch (cause) {
      show(`选择路径失败: ${cause instanceof Error ? cause.message : String(cause)}`);
    }
  }

  async function triggerOption(descriptor: EngineOptionDescriptor) {
    if (descriptor.option_type !== "button") return;
    try {
      await unwrap(await commands.engineTriggerButtonOption(descriptor.name));
      show(`已触发引擎选项「${descriptor.name}」`);
    } catch (cause) {
      show(`触发引擎选项失败: ${cause instanceof Error ? cause.message : String(cause)}`);
    }
  }

  async function downloadBuiltinEngine(profileId: string) {
    if (isDownloading.value) return;
    isDownloading.value = true;
    downloadProgress.value = {
      profileId,
      stage: "downloading",
      downloadedBytes: 0,
      totalBytes: null,
      percent: 0,
      speedBytesPerSec: 0,
      message: "正在发起 GitHub 在线下载请求...",
    };

    try {
      downloadUnlisten.value = await listenDownloadProgress((payload) => {
        if (payload.profileId === profileId) downloadProgress.value = payload;
      });
      const updated = await unwrap(await commands.engineDownloadBuiltin(profileId));
      preferences.saveEngineProfile(updated, { activate: false });
      selectedProfileId.value = updated.id;
      loadProfileToForm(updated.id);
      show("皮卡鱼 (Pikafish) 引擎已安装就绪，已自动绑定神经网络权重");
    } catch (cause) {
      await preferences.init();
      loadProfileToForm(profileId);
      show(`引擎获取失败: ${cause instanceof Error ? cause.message : String(cause)}`);
    } finally {
      isDownloading.value = false;
      downloadUnlisten.value?.();
      downloadUnlisten.value = null;
    }
  }

  async function applyEngineAndRestart() {
    saveProfileAndSync();
    if (!currentProfile.path.trim()) {
      show("当前引擎尚未下载或指定路径");
      return;
    }
    try {
      const config = engineConfigFromProfile(currentProfile);
      if (!config) {
        show("当前引擎尚未下载或指定路径");
        return;
      }
      await engine.start(config);
      preferences.setActiveEngineId(currentProfile.id);
      show(`引擎「${currentProfile.name}」已成功加载运行`);
    } catch (cause) {
      show(`引擎启动失败: ${cause instanceof Error ? cause.message : String(cause)}`);
    }
  }

  onScopeDispose(() => {
    downloadUnlisten.value?.();
    downloadUnlisten.value = null;
  });

  return {
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
  };
}
