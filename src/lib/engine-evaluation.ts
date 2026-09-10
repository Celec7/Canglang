/** 将红方视角的厘兵分转换为有界胜率 */
export function scoreToWinRate(score: number): number {
  return Math.min(0.99, Math.max(0.01, 1 / (1 + Math.exp(-score / 400))));
}

/** 保证极端引擎分值在紧凑评估图中仍然清晰可读 */
export function clampEvaluation(score: number): number {
  return Math.min(1000, Math.max(-1000, score));
}

export function formatEvaluation(score: number, mateIn: number | null): string {
  if (mateIn !== null) {
    if (mateIn > 0) return `红杀${mateIn}`;
    if (mateIn < 0) return `黑杀${Math.abs(mateIn)}`;
    return "绝杀";
  }
  return `${score >= 0 ? "+" : ""}${(score / 100).toFixed(2)}`;
}

export interface DetailedEvaluation {
  label: string;
  isMate: boolean;
  isRed: boolean;
}

/**
 * 将引擎输出的走子方相对分值转换为红方绝对视角（Red-Perspective）：
 * - 若当前轮到红方走，红优为正；
 * - 若当前轮到黑方走，引擎的正分代表黑优，需取反转为红方视角
 */
export function toRedPerspective(
  score: number,
  mateIn: number | null | undefined,
  redToMove: boolean
): { score: number; mateIn: number | null } {
  return {
    score: redToMove ? score : -score,
    mateIn: mateIn !== null && mateIn !== undefined ? (redToMove ? mateIn : -mateIn) : null,
  };
}

export function formatScoreDetailed(score: number, mateIn: number | null): DetailedEvaluation {
  if (mateIn !== null && mateIn !== undefined) {
    if (mateIn > 0) {
      return { label: `红杀 ${mateIn} 步`, isMate: true, isRed: true };
    }
    if (mateIn < 0) {
      return { label: `黑杀 ${Math.abs(mateIn)} 步`, isMate: true, isRed: false };
    }
    return { label: "绝杀", isMate: true, isRed: false };
  }

  const cp = (Math.abs(score) / 100).toFixed(2);
  const isRed = score >= 0;
  return {
    label: isRed ? `红优: +${cp}` : `黑优: +${cp}`,
    isMate: false,
    isRed,
  };
}

export function formatNps(nps: number): string {
  if (!Number.isFinite(nps) || nps <= 0) return "0K";
  if (nps >= 1_000_000) {
    return `${(nps / 1_000_000).toFixed(2)}M`;
  }
  return `${Math.round(nps / 1000)}K`;
}

export function formatElapsed(timeMs: number): string {
  if (!Number.isFinite(timeMs) || timeMs <= 0) return "0.0s";
  return `${(timeMs / 1000).toFixed(1)}s`;
}
