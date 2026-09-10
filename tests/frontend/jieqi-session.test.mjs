import assert from "node:assert/strict";
import { after, afterEach, before, beforeEach, test } from "node:test";
import { createPinia, disposePinia, setActivePinia } from "pinia";
import { createServer } from "vite";

let server;
let commands;
let originalCommands;
let useGameStore;
let positionViewToBoard;
let pinia;

const ok = (data) => ({ status: "ok", data });
const capability = (enabled, reason = null) => ({ enabled, reason });

function capabilities(overrides = {}) {
  return {
    move: capability(true), undo: capability(false, "no_history"), redo: capability(false, "no_future"),
    jump: capability(true), resign: capability(true), offer_draw: capability(false, "wrong_variant"),
    save_private: capability(false, "wrong_variant"), save_public: capability(false, "wrong_variant"),
    edit_annotations: capability(true), analyze: capability(true), query_book: capability(true),
    edit_position: capability(true), use_fen: capability(true), ...overrides,
  };
}

function xiangqiSnapshot(revision = "r0", moves = []) {
  const fen = "4k4/9/9/9/9/9/9/9/9/4K4 w";
  return {
    game_id: "game-1", revision, content_revision: `c-${moves.length}`,
    position: { variant: "xiangqi", fen }, start_position: { variant: "xiangqi", fen },
    head_ply: moves.length, current_ply: moves.length, source: "local",
    rules: { variant: "xiangqi", profile: "china2020" }, play_mode: "training",
    result: { type: "ongoing" }, in_check: false,
    history: moves.map((iccs, index) => ({ variant: "xiangqi", ply: {
      ply: index + 1, iccs, notation: iccs, mover: index % 2 ? "black" : "red",
      is_capture: false, is_check: false, fen,
    } })),
    xiangqi_assessment: { repetition_count: 1, repetition_explanation: null, rule_status: "ongoing", rule_explanation: null },
    capabilities: capabilities(), draw_offer: null,
  };
}

function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

before(async () => {
  server = await createServer({ server: { middlewareMode: true, watch: null, ws: false }, appType: "custom" });
  ({ commands } = await server.ssrLoadModule("/src/lib/ipc.ts"));
  originalCommands = { ...commands };
  ({ useGameStore } = await server.ssrLoadModule("/src/stores/game.ts"));
  ({ positionViewToBoard } = await server.ssrLoadModule("/src/lib/position-view.ts"));
});

beforeEach(() => {
  Object.assign(commands, originalCommands);
  globalThis.window = { setTimeout };
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: { getItem: () => null, setItem: () => {} },
  });
  pinia = createPinia();
  setActivePinia(pinia);
});

afterEach(() => {
  disposePinia(pinia);
  delete globalThis.localStorage;
  delete globalThis.window;
});
after(async () => server?.close());

test("揭棋公开位置映射不需要 FEN，也不携带真实身份", () => {
  const view = {
    variant: "jieqi",
    position: { turn: "red", pieces: [
      { state: "hidden", position: { row: 9, col: 0 }, color: "red", move_as: "rook" },
      { state: "revealed", position: { row: 8, col: 0 }, color: "red", kind: "pawn" },
    ] },
  };
  const mapped = positionViewToBoard(view);
  assert.equal(mapped.turn, "red");
  assert.equal(mapped.board[9][0], "jieqi:hidden:red:rook");
  assert.equal(mapped.board[8][0], "jieqi:revealed:red:pawn");
  assert.equal(JSON.stringify(mapped).includes("assigned"), false);
});

test("连续走子按快照版本串行发送", async () => {
  commands.sessionGet = async () => xiangqiSnapshot();
  const game = useGameStore();
  await game.init();
  const first = deferred();
  const calls = [];
  commands.sessionMove = async (token, iccs) => {
    calls.push({ token, iccs });
    if (calls.length === 1) return first.promise;
    return ok(xiangqiSnapshot("r2", ["a0a1", "a9a8"]));
  };

  const one = game.makeMove("a0a1");
  const two = game.makeMove("a9a8");
  await Promise.resolve();
  assert.equal(calls.length, 1);
  first.resolve(ok(xiangqiSnapshot("r1", ["a0a1"])));
  await Promise.all([one, two]);
  assert.equal(calls[0].token.expected_revision, "r0");
  assert.equal(calls[1].token.expected_revision, "r1");
  assert.equal(game.revision, "r2");
});

test("过期走子只刷新快照，不自动重试", async () => {
  commands.sessionGet = async () => xiangqiSnapshot("r0");
  const game = useGameStore();
  await game.init();
  let moveCalls = 0;
  commands.sessionMove = async () => {
    moveCalls++;
    return { status: "error", error: { code: "stale_session", message: "会话已更新" } };
  };
  commands.sessionGet = async () => xiangqiSnapshot("server-r1");
  await assert.rejects(game.makeMove("a0a1"), /会话已更新/);
  assert.equal(moveCalls, 1);
  assert.equal(game.revision, "server-r1");
});

test("迟到的目标响应不能应用到新版快照", async () => {
  commands.sessionGet = async () => xiangqiSnapshot("r0");
  const game = useGameStore();
  await game.init();
  const target = deferred();
  commands.sessionTargets = async () => target.promise;
  const pending = game.targets(9, 0);
  commands.sessionGet = async () => xiangqiSnapshot("r1");
  await game.refresh();
  target.resolve(ok(["a0a1"]));
  assert.deepEqual(await pending, []);
});
