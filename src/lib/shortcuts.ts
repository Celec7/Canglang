export interface ShortcutDefinition {
  id: string;
  category: "game" | "board" | "workspace" | "general";
  title: string;
  keys: string;
  description?: string;
}

export const SHORTCUT_DEFINITIONS: ShortcutDefinition[] = [
  // 对局与复盘
  {
    id: "undo-move",
    category: "game",
    title: "悔棋 / 上一步",
    keys: "[ / Ctrl+Z",
    description: "撤回至上一走步或悔棋",
  },
  {
    id: "redo-move",
    category: "game",
    title: "重做 / 下一步",
    keys: "] / Ctrl+Y",
    description: "前进至下一步走法",
  },
  {
    id: "first-move",
    category: "game",
    title: "起始局面",
    keys: "Home / Shift+[",
    description: "跳至棋局起始位置",
  },
  {
    id: "last-move",
    category: "game",
    title: "最新局面",
    keys: "End / Shift+]",
    description: "跳至对局最新着法",
  },
  {
    id: "toggle-playback",
    category: "game",
    title: "自动播放 / 暂停",
    keys: "Space",
    description: "在复盘时按步进自动演示或暂停",
  },
  {
    id: "new-game",
    category: "game",
    title: "新对局",
    keys: "Ctrl+N",
    description: "重置棋盘开启新对局",
  },
  {
    id: "open-manual",
    category: "game",
    title: "打开棋谱",
    keys: "Ctrl+O",
    description: "导入 PGN 或 XQF 棋谱",
  },
  {
    id: "open-fen",
    category: "game",
    title: "FEN 局面",
    keys: "Ctrl+Shift+F",
    description: "导入或导出局面 FEN 字符串",
  },

  // 棋盘与视角
  {
    id: "board-nav",
    category: "board",
    title: "光标导航",
    keys: "↑ ↓ ← →",
    description: "在棋盘 90 个交叉点之间游走",
  },
  {
    id: "board-select",
    category: "board",
    title: "选择 / 落子",
    keys: "Enter / Space",
    description: "选中光标处己方棋子，或落子到合法目标格",
  },
  {
    id: "board-cancel",
    category: "board",
    title: "取消选择",
    keys: "Escape",
    description: "清除当前选中棋子及落子提示",
  },
  {
    id: "flip-board",
    category: "board",
    title: "翻转棋盘视角",
    keys: "F",
    description: "在红方与黑方视角间快速切换",
  },
  {
    id: "toggle-coords",
    category: "board",
    title: "切换坐标显示",
    keys: "C",
    description: "开启或隐藏棋盘边缘路数与数字",
  },

  // 分析与工作区
  {
    id: "toggle-engine",
    category: "workspace",
    title: "开关引擎分析",
    keys: "E / Ctrl+E",
    description: "开启或停止后台引擎实时推算",
  },
  {
    id: "toggle-analysis-panel",
    category: "workspace",
    title: "切换分析面板",
    keys: "Ctrl+1 / Ctrl+B",
    description: "展开或收起中栏引擎与开局库监视区",
  },
  {
    id: "toggle-movelist-panel",
    category: "workspace",
    title: "切换着法列表",
    keys: "Ctrl+2 / Ctrl+J",
    description: "展开或收起右栏着法记录与复盘区",
  },
  {
    id: "toggle-sound",
    category: "workspace",
    title: "音效静音切换",
    keys: "M",
    description: "开启或静音走子提示音效",
  },

  // 通用与帮助
  {
    id: "open-settings",
    category: "general",
    title: "偏好设置",
    keys: "Ctrl+,",
    description: "打开外观、引擎与棋规设置",
  },
  {
    id: "show-shortcuts",
    category: "general",
    title: "快捷键速查",
    keys: "? / F1",
    description: "打开键盘快捷键全量参考弹窗",
  },
];

export const CATEGORY_NAMES: Record<ShortcutDefinition["category"], string> = {
  game: "对局与复盘",
  board: "棋盘与光标",
  workspace: "分析与工作区",
  general: "通用与帮助",
};

/**
 * 判断当前焦点事件是否应由当前控件或对话框自行处理
 * 全局快捷键不能抢占按钮的 Space 激活、菜单项操作或设置弹窗内的键盘输入
 */
export function isInputActive(target: EventTarget | null): boolean {
  if (!target || !(target instanceof HTMLElement)) return false;
  const tag = target.tagName.toLowerCase();
  return (
    tag === "input" ||
    tag === "textarea" ||
    tag === "select" ||
    target.isContentEditable ||
    target.getAttribute("role") === "textbox" ||
    target.closest('[role="dialog"]') !== null ||
    target.matches('button, a, [role="button"], [role="menuitem"], [role="switch"], [role="checkbox"], [role="combobox"], [role="slider"], [role="tab"]')
  );
}

/**
 * 适配系统平台（Mac 显示 ⌘，Windows/Linux 显示 Ctrl）
 */
export function formatShortcut(key: string): string {
  if (typeof navigator === "undefined") return key;
  const isMac = /Mac|iPhone|iPod|iPad/i.test(navigator.platform || navigator.userAgent);
  if (isMac) {
    return key.replace(/Ctrl\+/g, "⌘").replace(/Shift\+/g, "⇧");
  }
  return key;
}
