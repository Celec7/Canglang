import type { RuleExplanation, RuleStatus } from "@/bindings";

/**
 * 将英文对局结果转换为统一的中文展示文案
 */
export function resultLabel(result: string): string {
  const labels: Record<string, string> = {
    ongoing: "进行中",
    redwin: "红胜",
    blackwin: "黑胜",
    draw: "和棋",
  };
  return labels[result] ?? result;
}

/**
 * 将规则裁判状态转换为用户友好的简要标签
 */
export function statusLabel(status: RuleStatus, result: string): string {
  const labels: Record<string, string> = {
    ongoing: "进行中",
    checkmate: "将死",
    stalemate: "困毙",
    repetition_pending: "待裁判",
    prohibited_move_pending: "待变着",
    red_loss_by_rule: "红方判负",
    black_loss_by_rule: "黑方判负",
    draw_by_rule: "规则判和",
  };
  return labels[status] ?? resultLabel(result);
}

/**
 * 提取规则裁判或重复局面的详细提示说明
 */
export function ruleExplanationText(
  explanation: RuleExplanation | null | undefined,
  repetitionExplanation: string | null | undefined
): string | null {
  if (explanation?.message) return explanation.message;
  if (repetitionExplanation) return repetitionExplanation;
  return null;
}

/**
 * 将 1-based 的半回合步数（ply）转换为第几回合及红黑方
 * 例如：ply 1 -> { roundNumber: 1, side: "red" }
 *      ply 2 -> { roundNumber: 1, side: "black" }
 *      ply 3 -> { roundNumber: 2, side: "red" }
 */
export function formatPlyNumber(ply: number): { roundNumber: number; side: "red" | "black" } {
  const roundNumber = Math.floor((Math.max(1, ply) - 1) / 2) + 1;
  const side = ply % 2 === 1 ? "red" : "black";
  return { roundNumber, side };
}
