import assert from "node:assert/strict";
import { test } from "node:test";
import { CHESS_MOVE_EASE, createCubicBezier } from "../../src/lib/motion.ts";

test("CubicBezier 边界值与单调性", () => {
  const ease = createCubicBezier(0.16, 1, 0.3, 1);
  assert.equal(ease(0), 0);
  assert.equal(ease(1), 1);
  assert.equal(ease(-0.1), 0);
  assert.equal(ease(1.1), 1);

  // 中间点平滑单调递增
  let prev = 0;
  for (let i = 0.05; i <= 1; i += 0.05) {
    const curr = ease(i);
    assert.ok(curr >= prev, `at ${i}: curr ${curr} >= prev ${prev}`);
    prev = curr;
  }

  // 验证起点迅速加速，后半程平滑减速 (0.16, 1, 0.3, 1 特性：在 t=0.3 时位移已过大半)
  assert.ok(CHESS_MOVE_EASE(0.3) > 0.6);
});
