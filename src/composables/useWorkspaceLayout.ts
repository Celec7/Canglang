import { computed, onBeforeUnmount, onMounted, ref } from "vue";

export type WorkspaceLayoutMode = "phone-scroll" | "tablet-single" | "tablet-split" | "desktop-split";
export type WorkspaceSurface = "moves" | "analysis";

function layoutModeForWidth(width: number): WorkspaceLayoutMode {
  if (width < 600) return "phone-scroll";
  if (width < 900) return "tablet-single";
  if (width < 1280) return "tablet-split";
  return "desktop-split";
}

export function useWorkspaceLayout() {
  const initialWidth = typeof window !== "undefined" ? window.innerWidth : 1280;
  const layoutMode = ref<WorkspaceLayoutMode>(layoutModeForWidth(initialWidth));
  const desktopShowAnalysis = ref(layoutMode.value === "desktop-split");
  const desktopShowMoveList = ref(true);
  const tabletShowMoveList = ref(true);
  const activeSurface = ref<WorkspaceSurface | null>(null);
  const userToggledAnalysis = ref(false);

  const showAnalysis = computed(() => {
    if (layoutMode.value === "desktop-split") return desktopShowAnalysis.value;
    return activeSurface.value === "analysis";
  });

  const showMoveList = computed(() => {
    if (layoutMode.value === "desktop-split") return desktopShowMoveList.value;
    if (layoutMode.value === "tablet-split") return tabletShowMoveList.value;
    return activeSurface.value === "moves";
  });

  function toggleSurface(surface: WorkspaceSurface) {
    activeSurface.value = activeSurface.value === surface ? null : surface;
  }

  function toggleAnalysis() {
    if (layoutMode.value === "desktop-split") {
      userToggledAnalysis.value = true;
      desktopShowAnalysis.value = !desktopShowAnalysis.value;
      return;
    }
    toggleSurface("analysis");
  }

  function toggleMoveList() {
    if (layoutMode.value === "desktop-split") {
      desktopShowMoveList.value = !desktopShowMoveList.value;
      return;
    }
    if (layoutMode.value === "tablet-split") {
      tabletShowMoveList.value = !tabletShowMoveList.value;
      return;
    }
    toggleSurface("moves");
  }

  function openSurface(surface: WorkspaceSurface) {
    if (layoutMode.value === "desktop-split") return;
    activeSurface.value = surface;
  }

  function closeSurface() {
    activeSurface.value = null;
  }

  function handleResize() {
    if (typeof window === "undefined") return;
    const nextMode = layoutModeForWidth(window.innerWidth);
    if (nextMode !== layoutMode.value) {
      layoutMode.value = nextMode;
      activeSurface.value = null;
    }
    if (nextMode === "desktop-split" && !userToggledAnalysis.value) {
      desktopShowAnalysis.value = true;
    }
  }

  onMounted(() => {
    window.addEventListener("resize", handleResize);
  });

  onBeforeUnmount(() => {
    window.removeEventListener("resize", handleResize);
  });

  return {
    layoutMode,
    activeSurface,
    showAnalysis,
    showMoveList,
    toggleAnalysis,
    toggleMoveList,
    toggleSurface,
    openSurface,
    closeSurface,
  };
}
