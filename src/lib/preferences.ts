import type { CloudBookMode, EngineProfile } from "@/bindings";

export type { EngineProfile };
export type ThemeMode = "light" | "dark";
export type BoardOrientation = "red" | "black" | "follow_turn";
export type AnalysisModeType = "fixed_time" | "fixed_depth" | "fixed_nodes" | "infinite";
export type RuleProfileType = "china2020" | "asian2017";

export interface Preferences {
  theme: ThemeMode;
  boardOrientation: BoardOrientation;
  showCoordinates: boolean;
  showEngineArrow: boolean;
  animations: boolean;
  moveAnimationSeconds: number;
  soundEnabled: boolean;
  defaultRuleProfile: RuleProfileType;
  openingBookPaths: string[];
  cloudBookEnabled: boolean;
  cloudBookMode: CloudBookMode;
  engineProfiles: EngineProfile[];
  activeEngineId: string | null;
  analysisMode: AnalysisModeType;
  analysisLimitValue: number;
  multiPv: number;
}

export function getDefaultHardwareDefaults() {
  const cores =
    typeof navigator !== "undefined" && navigator.hardwareConcurrency
      ? navigator.hardwareConcurrency
      : 4;
  const threads = Math.max(1, Math.min(8, cores >= 4 ? cores - 1 : cores));
  const hashMb = 256;
  return { threads, hashMb };
}

export const DEFAULT_PREFERENCES: Preferences = {
  theme: "light",
  boardOrientation: "red",
  showCoordinates: true,
  showEngineArrow: true,
  animations: true,
  moveAnimationSeconds: 0.2,
  soundEnabled: true,
  defaultRuleProfile: "china2020",
  openingBookPaths: [],
  cloudBookEnabled: true,
  cloudBookMode: "hybrid",
  engineProfiles: [],
  activeEngineId: null,
  analysisMode: "fixed_time",
  analysisLimitValue: 1000,
  multiPv: 1,
};

export const STORAGE_KEY = "canglang.preferences";

export function loadPreferences(): Preferences {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return { ...DEFAULT_PREFERENCES };

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
  if (!parsed || typeof parsed !== "object") return { ...DEFAULT_PREFERENCES };

  const value = parsed as Partial<Preferences> & { defaultEnginePath?: string };
  // localStorage 中可能残留旧版本档案：补齐内置引擎相关字段，避免组件读到 undefined
  const profiles: EngineProfile[] = Array.isArray(value.engineProfiles)
    ? value.engineProfiles.map((profile) => ({
        ...profile,
        nnuePath: profile.nnuePath ?? null,
        optionOverrides: profile.optionOverrides ?? {},
        releaseRevision: profile.releaseRevision ?? "",
        installedRevision: profile.installedRevision ?? "",
        isBuiltin: profile.isBuiltin ?? false,
        downloadUrl: profile.downloadUrl ?? null,
        description: profile.description ?? null,
      }))
    : [];
  if (profiles.length === 0 && value.defaultEnginePath) {
    profiles.push({
      id: "default-engine",
      name: "默认象棋引擎",
      path: value.defaultEnginePath,
      protocol: "auto",
      nnuePath: null,
      optionOverrides: {},
      releaseRevision: "",
      installedRevision: "",
      isBuiltin: false,
      downloadUrl: null,
      description: null,
      ...getDefaultHardwareDefaults(),
    });
  }

  const openingBookPaths = Array.isArray(value.openingBookPaths)
    ? [...value.openingBookPaths].filter((p): p is string => typeof p === "string" && p.trim().length > 0)
    : [];

  return {
    ...DEFAULT_PREFERENCES,
    ...value,
    openingBookPaths,
    engineProfiles: profiles,
    activeEngineId: value.activeEngineId ?? (profiles[0]?.id ?? null),
    analysisMode: value.analysisMode ?? DEFAULT_PREFERENCES.analysisMode,
    analysisLimitValue: value.analysisLimitValue ?? DEFAULT_PREFERENCES.analysisLimitValue,
    multiPv: value.multiPv ?? DEFAULT_PREFERENCES.multiPv,
    moveAnimationSeconds: clampMoveAnimationSeconds(
      value.moveAnimationSeconds ?? DEFAULT_PREFERENCES.moveAnimationSeconds
    ),
  };
}

export function savePreferences(preferences: Preferences): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences));
}

export function clampMoveAnimationSeconds(value: number): number {
  return Math.min(1, Math.max(0.05, Number.isFinite(value) ? Math.round(value * 100) / 100 : DEFAULT_PREFERENCES.moveAnimationSeconds));
}
