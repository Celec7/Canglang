import { ref } from "vue";

const politeMessage = ref("");
const assertiveMessage = ref("");

let politeTimer: ReturnType<typeof setTimeout> | null = null;
let assertiveTimer: ReturnType<typeof setTimeout> | null = null;

export function useA11yAnnouncer() {
  function announce(message: string, priority: "polite" | "assertive" = "polite") {
    if (!message) return;

    if (priority === "assertive") {
      if (assertiveTimer) clearTimeout(assertiveTimer);
      assertiveMessage.value = "";
      assertiveTimer = setTimeout(() => {
        assertiveMessage.value = message;
      }, 50);
    } else {
      if (politeTimer) clearTimeout(politeTimer);
      politeMessage.value = "";
      politeTimer = setTimeout(() => {
        politeMessage.value = message;
      }, 50);
    }
  }

  return {
    politeMessage,
    assertiveMessage,
    announce,
  };
}
