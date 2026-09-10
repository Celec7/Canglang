import { usePreferencesStore } from "@/stores/preferences";

type SoundType = "move" | "capture" | "check" | "win";

const soundCache: Partial<Record<SoundType, HTMLAudioElement>> = {};

function getAudio(type: SoundType): HTMLAudioElement {
  if (!soundCache[type]) {
    soundCache[type] = new Audio(`/sounds/${type}.wav`);
  }
  return soundCache[type]!;
}

export function playSound(type: SoundType) {
  const preferences = usePreferencesStore();
  if (!preferences.soundEnabled) return;

  try {
    const audio = getAudio(type);
    audio.currentTime = 0;
    void audio.play().catch(() => undefined);
  } catch {
    // 忽略音频播放错误
  }
}
