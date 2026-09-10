import type { EngineConfig, EngineOptionValue, EngineProfile } from "@/bindings";

function optionValueToProtocol(value: EngineOptionValue): string {
  switch (value.kind) {
    case "bool":
    case "integer":
    case "float":
    case "enum":
    case "string":
    case "path":
      return String(value.value);
  }
}

/** 将持久化 Profile 唯一地转换为引擎启动配置 */
export function engineConfigFromProfile(profile: EngineProfile): EngineConfig | null {
  const path = profile.path.trim();
  if (!path) return null;

  const options: Record<string, string> = {};
  if (profile.threads) options.Threads = String(profile.threads);
  if (profile.hashMb) options.Hash = String(profile.hashMb);
  for (const [name, value] of Object.entries(profile.optionOverrides ?? {})) {
    options[name] = optionValueToProtocol(value);
  }

  return {
    path,
    protocol: profile.protocol,
    options: Object.keys(options).length > 0 ? options : null,
    nnue_path: profile.nnuePath?.trim() || null,
  };
}
