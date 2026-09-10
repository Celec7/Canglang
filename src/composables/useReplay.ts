import { ref } from "vue";
import { useGameStore } from "@/stores/game";

const isPlaying = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;

export function useReplay() {
  const game = useGameStore();

  function stopPlayback() {
    isPlaying.value = false;
    if (timer) {
      clearTimeout(timer);
      timer = undefined;
    }
  }

  async function playStep() {
    if (!isPlaying.value || !game.capabilities.redo.enabled) {
      stopPlayback();
      return;
    }
    await game.redo();
    timer = setTimeout(() => void playStep(), 600);
  }

  function togglePlayback() {
    if (isPlaying.value) {
      stopPlayback();
      return;
    }
    if (!game.capabilities.redo.enabled) return;
    isPlaying.value = true;
    void playStep();
  }

  return {
    isPlaying,
    togglePlayback,
    stopPlayback,
  };
}
