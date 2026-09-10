import type { AnalysisConfig, RootMoveConstraint, RuleProfile } from "@/bindings";

export type AnalysisAction = "move" | "variation" | null;

export interface AnalysisContext {
  analysisSessionId: string;
  startFen: string;
  fen: string;
  history: string[];
  ruleProfile: RuleProfile;
  constraint: RootMoveConstraint;
  retryCount: number;
  action: AnalysisAction;
}

export function isPositionCurrent(
  context: Pick<AnalysisContext, "fen" | "history">,
  currentFen: string,
  currentHistory: readonly string[]
): boolean {
  return (
    context.fen === currentFen &&
    context.history.length === currentHistory.length &&
    context.history.every((move, index) => move === currentHistory[index])
  );
}

export function autoMoveConfig(config: AnalysisConfig): AnalysisConfig {
  return config.mode === "infinite"
    ? { ...config, mode: "fixed_time", value: 1500 }
    : { ...config };
}

export function shouldPlayBestMove(action: AnalysisAction, controlledTurn: boolean): boolean {
  return action === "move" || controlledTurn;
}
