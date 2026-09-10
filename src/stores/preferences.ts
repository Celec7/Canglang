import { defineStore } from "pinia";
import { reactive, ref, toRefs, watch } from "vue";
import {
  clampMoveAnimationSeconds,
  loadPreferences,
  savePreferences,
  STORAGE_KEY,
  type AnalysisModeType,
  type BoardOrientation,
  type EngineProfile,
  type Preferences,
  type RuleProfileType,
  type ThemeMode,
} from "@/lib/preferences";
import type { AppConfig, CloudBookMode, ConfigLocationInfo } from "@/bindings";
import { commands, unwrap } from "@/lib/ipc";

export const usePreferencesStore = defineStore("preferences", () => {
  const preferences = reactive<Preferences>(loadPreferences());
  const locationInfo = ref<ConfigLocationInfo | null>(null);
  const persistenceError = ref<string | null>(null);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  async function saveToDisk() {
    try {
      const payload: AppConfig = {
        theme: preferences.theme,
        boardOrientation: preferences.boardOrientation,
        showCoordinates: preferences.showCoordinates,
        showEngineArrow: preferences.showEngineArrow,
        animations: preferences.animations,
        moveAnimationSeconds: preferences.moveAnimationSeconds,
        soundEnabled: preferences.soundEnabled,
        defaultRuleProfile: preferences.defaultRuleProfile,
        openingBookPaths: [...preferences.openingBookPaths],
        cloudBookEnabled: preferences.cloudBookEnabled,
        cloudBookMode: preferences.cloudBookMode,
        engineProfiles: preferences.engineProfiles.map((p) => ({
          ...p,
          threads: p.threads ?? null,
          hashMb: p.hashMb ?? null,
        })),
        activeEngineId: preferences.activeEngineId,
        analysisMode: preferences.analysisMode,
        analysisLimitValue: preferences.analysisLimitValue,
        multiPv: preferences.multiPv,
      };
      await unwrap(await commands.configSave(payload));
      persistenceError.value = null;
    } catch (cause) {
      persistenceError.value = cause instanceof Error ? cause.message : String(cause);
    }
  }

  // 监听偏好变动：备份至 localStorage 并防抖写入 Rust config.json (原子写入)
  watch(
    preferences,
    (value) => {
      savePreferences(value);
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        void saveToDisk();
      }, 150);
    },
    { deep: true }
  );

  async function init() {
    try {
      // 1. 获取物理路径及便携模式状态
      locationInfo.value = await unwrap(await commands.configGetLocation());

      // 2. 从 Rust 端加载 config.json
      const diskConfig = await unwrap(await commands.configLoad());

      // 3. 平滑迁移：仅在磁盘上还没有 config.json、且 localStorage 有旧数据时，
      //    才用前端旧配置初始化磁盘配置。默认配置本身已含内置引擎，不能用
      //    “档案列表为空”来判断首次运行
      const localRaw = typeof localStorage !== "undefined" ? localStorage.getItem(STORAGE_KEY) : null;
      if (locationInfo.value && !locationInfo.value.exists && localRaw) {
        const localPrefs = loadPreferences();
        Object.assign(preferences, localPrefs);
        await saveToDisk();
      } else {
        // 以磁盘持久化配置为准同步
        preferences.theme = (diskConfig.theme as ThemeMode) || preferences.theme;
        preferences.boardOrientation = (diskConfig.boardOrientation as BoardOrientation) || preferences.boardOrientation;
        preferences.showCoordinates = diskConfig.showCoordinates;
        preferences.showEngineArrow = diskConfig.showEngineArrow;
        preferences.animations = diskConfig.animations;
        if (typeof diskConfig.moveAnimationSeconds === "number") {
          preferences.moveAnimationSeconds = diskConfig.moveAnimationSeconds;
        }
        preferences.soundEnabled = diskConfig.soundEnabled;
        preferences.defaultRuleProfile = (diskConfig.defaultRuleProfile as RuleProfileType) || preferences.defaultRuleProfile;
        preferences.openingBookPaths = diskConfig.openingBookPaths || [];
        preferences.cloudBookEnabled = diskConfig.cloudBookEnabled;
        preferences.cloudBookMode = diskConfig.cloudBookMode || "hybrid";
        preferences.engineProfiles = (diskConfig.engineProfiles as EngineProfile[]) || [];
        preferences.activeEngineId = diskConfig.activeEngineId ?? null;
        preferences.analysisMode = (diskConfig.analysisMode as AnalysisModeType) || preferences.analysisMode;
        preferences.analysisLimitValue = diskConfig.analysisLimitValue || preferences.analysisLimitValue;
        preferences.multiPv = diskConfig.multiPv || preferences.multiPv;
      }
    } catch {
      // 降级使用现有内存配置
    }
  }

  function setTheme(theme: ThemeMode) {
    preferences.theme = theme;
  }

  function setBoardOrientation(orientation: BoardOrientation) {
    preferences.boardOrientation = orientation;
  }

  function setMoveAnimationSeconds(seconds: number) {
    preferences.moveAnimationSeconds = clampMoveAnimationSeconds(seconds);
  }

  function setSoundEnabled(enabled: boolean) {
    preferences.soundEnabled = enabled;
  }

  function setDefaultRuleProfile(profile: RuleProfileType) {
    preferences.defaultRuleProfile = profile;
  }

  function setEngineProfiles(profiles: EngineProfile[]) {
    preferences.engineProfiles = profiles;
  }

  function saveEngineProfile(profile: EngineProfile, options: { activate?: boolean } = {}) {
    const idx = preferences.engineProfiles.findIndex((p) => p.id === profile.id);
    if (idx >= 0) {
      preferences.engineProfiles[idx] = { ...profile };
    } else {
      preferences.engineProfiles.push({ ...profile });
    }
    if (options.activate ?? true) preferences.activeEngineId = profile.id;
  }

  function removeEngineProfile(id: string) {
    preferences.engineProfiles = preferences.engineProfiles.filter((p) => p.id !== id);
    if (preferences.activeEngineId === id) {
      preferences.activeEngineId = preferences.engineProfiles[0]?.id ?? null;
    }
  }

  function setActiveEngineId(id: string | null) {
    preferences.activeEngineId = id;
  }

  function setAnalysisSettings(mode: AnalysisModeType, limitValue: number, multiPv: number) {
    preferences.analysisMode = mode;
    preferences.analysisLimitValue = limitValue;
    preferences.multiPv = multiPv;
  }

  function addOpeningBookPath(path: string) {
    const trimmed = path.trim();
    if (!trimmed) return;
    if (!preferences.openingBookPaths.includes(trimmed)) {
      preferences.openingBookPaths.push(trimmed);
    }
  }

  function removeOpeningBookPath(path: string) {
    preferences.openingBookPaths = preferences.openingBookPaths.filter((p) => p !== path);
  }

  function clearOpeningBookPaths() {
    preferences.openingBookPaths = [];
  }

  function setCloudBookEnabled(enabled: boolean) {
    preferences.cloudBookEnabled = enabled;
  }

  function setCloudBookMode(mode: CloudBookMode) {
    preferences.cloudBookMode = mode;
  }

  return {
    ...toRefs(preferences),
    locationInfo,
    persistenceError,
    init,
    saveToDisk,
    setTheme,
    setBoardOrientation,
    setMoveAnimationSeconds,
    setSoundEnabled,
    setDefaultRuleProfile,
    setEngineProfiles,
    saveEngineProfile,
    removeEngineProfile,
    setActiveEngineId,
    setAnalysisSettings,
    addOpeningBookPath,
    removeOpeningBookPath,
    clearOpeningBookPaths,
    setCloudBookEnabled,
    setCloudBookMode,
  };
});
