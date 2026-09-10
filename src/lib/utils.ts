// shadcn-vue 的 `cn` 辅助：合并条件类名，并在经过 clsx 后用 tailwind-merge 解析 Tailwind 类名冲突
import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
