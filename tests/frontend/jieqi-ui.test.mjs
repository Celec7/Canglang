import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { after, afterEach, before, beforeEach, test } from "node:test";
import { createPinia, disposePinia, setActivePinia } from "pinia";
import { createServer } from "vite";

let server;
let commands;
let originalCommands;
let useGameStore;
let useBoard;
let useToast;
let jieqiPieceName;
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
  ({ useBoard } = await server.ssrLoadModule("/src/composables/useBoard.ts"));
  ({ useToast } = await server.ssrLoadModule("/src/composables/useToast.ts"));
  ({ jieqiPieceName, pieceGlyph } = await server.ssrLoadModule("/src/lib/chess.ts"));
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

test("暗子视觉无文字而读屏名称保留阵营与公开首步角色", () => {
  const token = "jieqi:hidden:red:rook";
  assert.equal(pieceGlyph(token), "");
  const board = Array.from({ length: 10 }, () => Array(9).fill(null));
  board[9][0] = token;
  const label = formatTraditionalSquareLabel({ row: 9, col: 0, board });
  assert.match(label, /红方 暗子/);
  assert.match(label, /首步按車行走/);
  assert.doesNotMatch(label, /真实|身份|assigned/i);
});

test("揭棋公开角色按阵营显示中国象棋棋子名", () => {
  assert.equal(jieqiPieceName("pawn", "red"), "兵");
  assert.equal(jieqiPieceName("pawn", "black"), "卒");
  assert.equal(jieqiPieceName("king", "red"), "帥");
  assert.equal(jieqiPieceName("king", "black"), "將");
  assert.equal(jieqiPieceName("rook", "red"), "車");
});

test("暗子复用普通棋子面且不绘制遮罩、虚线或可见文字", async () => {
  const board = await readFile(new URL("../../src/components/ChessBoard.vue", import.meta.url), "utf8");
  const styles = await readFile(new URL("../../src/assets/main.css", import.meta.url), "utf8");
  assert.match(board, /v-if="!isHiddenJieqi\(p\)"/);
  assert.doesNotMatch(board, /jieqi-piece-back|stroke-dasharray|首步\{\{/);
  assert.doesNotMatch(styles, /jieqi-piece-back/);
});

test("揭子动画是短促淡入且减少动画设置下禁用", async () => {
  const styles = await readFile(new URL("../../src/assets/main.css", import.meta.url), "utf8");
  assert.match(styles, /animation: jieqi-reveal 120ms ease-out/);
  assert.doesNotMatch(styles, /brightness\(/);
  assert.match(styles, /prefers-reduced-motion: reduce[\s\S]*?jieqi-piece--revealed \{ animation: none; \}/);
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

test("送将候选保持高亮并在尝试落子时通过 toast 说明不可走", async () => {
  commands.sessionGet = async () => snapshot();
  commands.sessionTargets = async () => ok(["a0b0"]);
  commands.sessionMove = async () => ({
    status: "error",
    error: { code: "illegal_move", message: "该走法会导致送将，不能走" },
  });
  const game = useGameStore();
  await game.init();
  const interaction = useBoard();

  await interaction.onSquareClick(9, 0);
  assert.deepEqual(interaction.candidateTargets.value, [[9, 1]]);
  await interaction.onSquareClick(9, 1);

  assert.equal(useToast().messages.value.at(-1)?.message, "该走法会导致送将，不能走");
  assert.deepEqual(interaction.selected.value, [9, 0]);
});

test("应用入口用新局弹窗，并以能力切换揭棋信息面板", async () => {
  const app = await readFile(new URL("../../src/App.vue", import.meta.url), "utf8");
  assert.match(app, /NewGameDialog/);
  assert.match(app, /game\.capabilities\.analyze\.enabled/);
  assert.match(app, /JieqiInfoPanel/);
});
