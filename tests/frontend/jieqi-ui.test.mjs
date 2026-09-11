import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { after, afterEach, before, beforeEach, test } from "node:test";
import { createPinia, disposePinia, setActivePinia } from "pinia";
import { createServer } from "vite";

let server;
let commands;
let originalCommands;
let useGameStore;
let pieceGlyph;
let formatTraditionalSquareLabel;
let pinia;

const ok = (data) => ({ status: "ok", data });
const capability = (enabled, reason = null) => ({ enabled, reason });

function snapshot(revision = "r0", drawOffer = null) {
  const pieces = [
    { state: "hidden", position: { row: 9, col: 0 }, color: "red", move_as: "rook" },
    { state: "revealed", position: { row: 0, col: 4 }, color: "black", kind: "king" },
  ];
  return {
    game_id: "jieqi-1", revision, content_revision: revision,
    position: { variant: "jieqi", position: { turn: "red", pieces } },
    start_position: { variant: "jieqi", position: { turn: "red", pieces } },
    head_ply: 0, current_ply: 0, source: "local", rules: { variant: "jieqi_casual_v1" },
    play_mode: "duel", result: { type: "ongoing" }, in_check: false, history: [],
    xiangqi_assessment: null,
    capabilities: {
      move: capability(!drawOffer, drawOffer ? "pending_draw" : null),
      undo: capability(false, "duel_policy"), redo: capability(false, "duel_policy"),
      jump: capability(true), resign: capability(true), offer_draw: capability(!drawOffer, drawOffer ? "pending_draw" : null),
      save_private: capability(!drawOffer, drawOffer ? "pending_draw" : null),
      save_public: capability(!drawOffer, drawOffer ? "pending_draw" : null),
      edit_annotations: capability(true), analyze: capability(false, "wrong_variant"),
      query_book: capability(false, "wrong_variant"), edit_position: capability(false, "wrong_variant"),
      use_fen: capability(false, "wrong_variant"),
    },
    draw_offer: drawOffer,
  };
}

before(async () => {
  server = await createServer({ server: { middlewareMode: true, watch: null, ws: false }, appType: "custom" });
  ({ commands } = await server.ssrLoadModule("/src/lib/ipc.ts"));
  originalCommands = { ...commands };
  ({ useGameStore } = await server.ssrLoadModule("/src/stores/game.ts"));
  ({ pieceGlyph } = await server.ssrLoadModule("/src/lib/chess.ts"));
  ({ formatTraditionalSquareLabel } = await server.ssrLoadModule("/src/lib/a11y.ts"));
});

beforeEach(() => {
  Object.assign(commands, originalCommands);
  globalThis.window = { setTimeout };
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: { getItem: () => null, setItem: () => {} } });
  pinia = createPinia();
  setActivePinia(pinia);
});

afterEach(() => {
  disposePinia(pinia);
  delete globalThis.localStorage;
  delete globalThis.window;
});
after(async () => server?.close());

test("暗子字形和读屏名称只包含阵营与公开首步角色", () => {
  const token = "jieqi:hidden:red:rook";
  assert.equal(pieceGlyph(token), "暗");
  const board = Array.from({ length: 10 }, () => Array(9).fill(null));
  board[9][0] = token;
  const label = formatTraditionalSquareLabel({ row: 9, col: 0, board });
  assert.match(label, /红方 暗/);
  assert.match(label, /首步按车行走/);
  assert.doesNotMatch(label, /真实|身份|assigned/i);
});

test("新揭棋对局通过统一会话命令携带对弈方式", async () => {
  commands.sessionGet = async () => snapshot();
  const game = useGameStore();
  await game.init();
  let requested;
  commands.sessionNew = async (_token, options) => {
    requested = options;
    return ok({ ...snapshot("r1"), play_mode: options.play_mode });
  };
  await game.newSession({ variant: "jieqi", play_mode: "training" });
  assert.deepEqual(requested, { variant: "jieqi", play_mode: "training" });
  assert.equal(game.playMode, "training");
});

test("求和请求、拒绝和取消均使用最新会话版本", async () => {
  commands.sessionGet = async () => snapshot();
  const game = useGameStore();
  await game.init();
  const tokens = [];
  commands.sessionOfferDraw = async (token) => {
    tokens.push(token.expected_revision);
    return ok(snapshot("r1", { id: "offer-1", proposer: "red" }));
  };
  commands.sessionCancelDraw = async (token) => {
    tokens.push(token.expected_revision);
    return ok(snapshot("r2"));
  };
  await game.offerDraw("red");
  assert.equal(game.snapshot.draw_offer.id, "offer-1");
  await game.cancelDraw("offer-1", "red");
  assert.deepEqual(tokens, ["r0", "r1"]);
});

test("应用入口用新局弹窗，并以能力切换揭棋信息面板", async () => {
  const app = await readFile(new URL("../../src/App.vue", import.meta.url), "utf8");
  assert.match(app, /NewGameDialog/);
  assert.match(app, /game\.capabilities\.analyze\.enabled/);
  assert.match(app, /JieqiInfoPanel/);
});
