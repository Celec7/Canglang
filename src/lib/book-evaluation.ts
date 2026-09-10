export interface ParsedEvaluation {
  label: string;
  badgeClass: string;
  branchesText?: string;
  tooltip: string;
}

export function parseBookEvaluation(note: string | null): ParsedEvaluation {
  if (!note) {
    return {
      label: "-",
      badgeClass: "text-muted-foreground/60 border-transparent bg-transparent",
      tooltip: "无特定评价或分支信息",
    };
  }

  const trimmed = note.trim();

  // 1. 残局库判定 (W / D / L)
  if (trimmed.startsWith("W")) {
    const match = trimmed.match(/(\d+)/);
    const steps = match ? `剩 ${match[1]} 步杀` : "必胜";
    return {
      label: "必胜",
      badgeClass: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border-emerald-500/30 font-semibold",
      branchesText: steps,
      tooltip: `残局胜棋：官胜局面${match ? `，最快 ${match[1]} 步将死对方` : ""}`,
    };
  }

  if (trimmed.startsWith("D")) {
    return {
      label: "官和",
      badgeClass: "bg-muted text-muted-foreground border-border",
      branchesText: "和棋",
      tooltip: "残局和棋：双方正常应对下为官和局面",
    };
  }

  if (trimmed.startsWith("L")) {
    const match = trimmed.match(/(\d+)/);
    const steps = match ? `剩 ${match[1]} 步` : "必败";
    return {
      label: "必败",
      badgeClass: "bg-rose-500/15 text-rose-600 dark:text-rose-400 border-rose-500/30 font-semibold",
      branchesText: steps,
      tooltip: `残局败棋：官败局面${match ? `，预计 ${match[1]} 步内被将死` : ""}`,
    };
  }

  // 2. 常规开局库判定 (! / * / ?)
  const branchMatch = trimmed.match(/\((\d+)-(\d+)\)/);
  const branchesText = branchMatch
    ? `${parseInt(branchMatch[1], 10)}已知 / ${parseInt(branchMatch[2], 10)}推荐`
    : undefined;

  if (trimmed.startsWith("!")) {
    return {
      label: "绝佳",
      badgeClass: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border-emerald-500/30 font-semibold",
      branchesText,
      tooltip: `绝佳着法（评分高，胜率极佳）${branchMatch ? `。云端收录 ${branchMatch[1]} 个已知应手，推荐其中 ${branchMatch[2]} 种可行后续走法` : ""}`,
    };
  }

  if (trimmed.startsWith("*")) {
    return {
      label: "可行",
      badgeClass: "bg-sky-500/15 text-sky-600 dark:text-sky-400 border-sky-500/30",
      branchesText,
      tooltip: `平稳可行着法${branchMatch ? `。云端收录 ${branchMatch[1]} 个已知应手，推荐其中 ${branchMatch[2]} 种可行后续走法` : ""}`,
    };
  }

  if (trimmed.startsWith("?")) {
    return {
      label: "劣手",
      badgeClass: "bg-amber-500/15 text-amber-600 dark:text-amber-400 border-amber-500/30",
      branchesText,
      tooltip: `错漏劣手（评分偏低，易入劣势）${branchMatch ? `。云端已知 ${branchMatch[1]} 种应手中仅 ${branchMatch[2]} 种相对可用` : ""}`,
    };
  }

  // 3. 普通文本（如本地开局库批注）
  return {
    label: trimmed,
    badgeClass: "bg-muted/80 text-muted-foreground border-border",
    tooltip: trimmed,
  };
}
