import type { InjectionKey } from "vue";

export interface TabsContext {
  active: () => string;
  setActive: (value: string) => void;
}

export const TabsInjectionKey: InjectionKey<TabsContext> = Symbol("tabs");
