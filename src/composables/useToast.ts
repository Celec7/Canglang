import { ref } from "vue";

export interface ToastMessage {
  id: number;
  message: string;
}

const messages = ref<ToastMessage[]>([]);
let nextId = 1;

export function useToast() {
  function show(message: string, durationMs = 2200) {
    const id = nextId++;
    messages.value.push({ id, message });
    window.setTimeout(() => {
      messages.value = messages.value.filter((toast) => toast.id !== id);
    }, durationMs);
  }

  return { messages, show };
}
