import assert from "node:assert/strict";
import { after, afterEach, before, beforeEach, test } from "node:test";
import { createPinia, disposePinia, setActivePinia } from "pinia";
import { createServer } from "vite";

let server;
let commands;
let originalCommands;
let useGameStore;
let useManualStore;
let pinia;

const ok = (data) => ({ status: "ok", data });
const capability = (enabled, reason = null) => ({ enabled, reason });

function jieqiSnapshot({
  gameId = "jieqi-1",
  revision = "r0",
  contentRevision = "c0",
  moves = [],
  currentPly = moves.length,
  source = "local",
} = {}) {
  const pieces = [
    { state: "revealed", position: { row: 0, col: 4 }, color: "black", kind: "king" },
    { state: "revealed", position: { row: 9, col: 4 }, color: "red", kind: "king" },
  ];
  const writable = source === "local";
  return {
    game_id: gameId,
    revision,
    content_revision: contentRevision,
    position: { variant: "jieqi", position: { turn: currentPly % 2 ? "black" : "red", pieces } },
    start_position: { variant: "jieqi", position: { turn: "red", pieces } },
    head_ply: moves.length,
    current_ply: currentPly,
    source,
    rules: { variant: "jieqi_casual_v1" },
    play_mode: "training",
    result: { type: "ongoing" },
    in_check: false,
    history: moves.map((iccs, index) => ({ variant: "jieqi", ply: {
      ply: index + 1,
      iccs,
      notation: `揭棋${index + 1}`,
      mover: index % 2 ? "black" : "red",
      revealed: "pawn",
      captured: { type: "none" },
      is_check: false,
    } })),
    xiangqi_assessment: null,
    capabilities: {
      move: capability(writable), undo: capability(writable && currentPly > 0),
      redo: capability(writable && currentPly < moves.length), jump: capability(true),
      resign: capability(writable), offer_draw: capability(writable),
      save_private: capability(writable, writable ? null : "read_only"),
      save_public: capability(true), edit_annotations: capability(true),
      analyze: capability(false, "wrong_variant"), query_book: capability(false, "wrong_variant"),
      edit_position: capability(false, "wrong_variant"), use_fen: capability(false, "wrong_variant"),
    },
    draw_offer: null,
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
  ({ useManualStore } = await server.ssrLoadModule("/src/stores/manual.ts"));
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

async function prepare(options = {}) {
  commands.sessionGet = async () => jieqiSnapshot(options);
  const game = useGameStore();
  await game.refresh();
  const manual = useManualStore();
  manual.clear();
  return { game, manual };
}

test("揭棋文档使用公开编辑分支，走子不创建普通 ChessManual 树", async () => {
  const { game, manual } = await prepare();
  commands.sessionMove = async () => ok(jieqiSnapshot({
    revision: "r1", contentRevision: "c1", moves: ["a3a4"],
  }));
  await game.makeMove("a3a4");

  assert.equal(manual.documentKind, "private_game");
  assert.equal(manual.manual, null);
  assert.equal(manual.dirty, true);
  assert.equal(manual.annotationForPly(1), null);
});

test("训练改走截断主线时同步删除越界备注", async () => {
  const { game, manual } = await prepare({
    contentRevision: "c3", revision: "r3", moves: ["a3a4", "a6a5", "c3c4"], currentPly: 1,
  });
  manual.updateJieqiComment(1, "保留");
  manual.updateJieqiComment(3, "应删除");
  commands.jieqiDocumentSave = async (_token, _path, kind, _meta, _notes, editRevision) => ok({
    game_id: game.gameId, content_revision: game.contentRevision, edit_revision: editRevision, kind,
  });
  await manual.saveJieqi("training.cjq", "private_game");
  assert.equal(manual.dirty, false);

  commands.sessionMove = async () => ok(jieqiSnapshot({
    revision: "r4", contentRevision: "c4", moves: ["a3a4", "e6e5"], currentPly: 2,
  }));
  await game.makeMove("e6e5");
  assert.equal(manual.annotationForPly(1), "保留");
  assert.equal(manual.annotationForPly(3), null);
  assert.equal(manual.dirty, true);
});

test("保存响应必须同时匹配对局、内容、编辑修订与原生种类", async () => {
  const { game, manual } = await prepare();
  manual.updateJieqiComment(0, "保存版本");
  const response = deferred();
  let request;
  commands.jieqiDocumentSave = async (...args) => {
    request = args;
    return response.promise;
  };
  const saving = manual.saveJieqi("private.cjq", "private_game");
  manual.updateJieqiComment(0, "保存期间的新编辑");
  response.resolve(ok({
    game_id: game.gameId,
    content_revision: game.contentRevision,
    edit_revision: request[5],
    kind: "private_game",
  }));
  await saving;
  assert.equal(manual.dirty, true);
  assert.equal(manual.annotationForPly(0), "保存期间的新编辑");

  commands.jieqiDocumentSave = async (_token, _path, kind, _meta, _notes, editRevision) => ok({
    game_id: game.gameId, content_revision: game.contentRevision, edit_revision: editRevision, kind,
  });
  await manual.saveJieqi("private.cjq", "private_game");
  assert.equal(manual.dirty, false);
});

test("保存期间继续走棋时旧内容回执不能清 dirty", async () => {
  const { game, manual } = await prepare();
  manual.updateJieqiComment(0, "保存版本");
  const response = deferred();
  let requestedEditRevision;
  commands.jieqiDocumentSave = async (_token, _path, _kind, _meta, _notes, editRevision) => {
    requestedEditRevision = editRevision;
    return response.promise;
  };
  const saving = manual.saveJieqi("private.cjq", "private_game");
  commands.sessionMove = async () => ok(jieqiSnapshot({
    revision: "r1", contentRevision: "c1", moves: ["a3a4"],
  }));
  await game.makeMove("a3a4");
  response.resolve(ok({
    game_id: "jieqi-1", content_revision: "c0",
    edit_revision: requestedEditRevision, kind: "private_game",
  }));
  await saving;
  assert.equal(game.contentRevision, "c1");
  assert.equal(manual.dirty, true);
});

test("私有局公开导出不清 dirty，公开文档原生保存可以清除", async () => {
  const { game, manual } = await prepare();
  manual.updateJieqiComment(0, "待保存");
  commands.jieqiDocumentSave = async (_token, _path, kind, _meta, _notes, editRevision) => ok({
    game_id: game.gameId, content_revision: game.contentRevision, edit_revision: editRevision, kind,
  });
  await manual.saveJieqi("public.cjq", "public_replay");
  assert.equal(manual.dirty, true);

  const publicSnapshot = jieqiSnapshot({ gameId: "public-2", source: "public_replay" });
  commands.jieqiDocumentOpen = async () => ok({
    snapshot: publicSnapshot,
    document: { kind: "public_replay", metadata: { ...manual.jieqiMetadata }, annotations: { 0: "公开" } },
  });
  assert.equal(await manual.openJieqi("public.cjq"), true);
  manual.updateJieqiComment(0, "公开编辑");
  await manual.saveJieqi("public.cjq", "public_replay");
  assert.equal(manual.dirty, false);
});

test("加载失败保留当前路径、元数据、备注与棋局", async () => {
  const { game, manual } = await prepare();
  manual.documentPath = "current.cjq";
  manual.jieqiMetadata.title = "当前文档";
  manual.updateJieqiComment(0, "当前备注");
  const before = {
    gameId: game.gameId,
    path: manual.documentPath,
    title: manual.jieqiMetadata.title,
    note: manual.annotationForPly(0),
  };
  commands.jieqiDocumentOpen = async () => ({
    status: "error", error: { code: "invalid_input", message: "损坏的揭棋文件" },
  });

  assert.equal(await manual.openJieqi("broken.cjq"), false);
  assert.deepEqual({
    gameId: game.gameId,
    path: manual.documentPath,
    title: manual.jieqiMetadata.title,
    note: manual.annotationForPly(0),
  }, before);
  assert.match(manual.error, /损坏/);
});

test("未保存保护支持取消、放弃以及等待保存完成", async () => {
  const { game, manual } = await prepare();
  manual.updateJieqiComment(0, "待处理");

  const cancelled = manual.confirmDiscard();
  await manual.resolveDiscard("cancel");
  assert.equal(await cancelled, false);

  const discarded = manual.confirmDiscard();
  await manual.resolveDiscard("discard");
  assert.equal(await discarded, true);

  manual.documentPath = "native.cjq";
  const response = deferred();
  commands.jieqiDocumentSave = async (_token, _path, kind, _meta, _notes, editRevision) => {
    const receipt = await response.promise;
    return ok({ ...receipt, edit_revision: editRevision, kind });
  };
  const guarded = manual.confirmDiscard();
  const resolving = manual.resolveDiscard("save");
  assert.equal(manual.resolvingDiscard, true);
  response.resolve({ game_id: game.gameId, content_revision: game.contentRevision });
  await resolving;
  assert.equal(await guarded, true);
  assert.equal(manual.dirty, false);
});

test("保护流程中的保存失败保持当前工作并阻止后续替换", async () => {
  const { manual } = await prepare();
  manual.updateJieqiComment(0, "不能丢失");
  manual.documentPath = "native.cjq";
  commands.jieqiDocumentSave = async () => ({
    status: "error", error: { code: "io_failure", message: "磁盘已满" },
  });

  const guarded = manual.confirmDiscard();
  await manual.resolveDiscard("save");
  assert.equal(manual.discardPromptOpen, true);
  assert.equal(manual.dirty, true);
  assert.match(manual.error, /磁盘已满/);
  await manual.resolveDiscard("cancel");
  assert.equal(await guarded, false);
});

test("新局请求在用户取消时不调用后端，放弃后才继续", async () => {
  const { game, manual } = await prepare();
  manual.updateJieqiComment(0, "待保存");
  let calls = 0;
  commands.sessionNew = async () => {
    calls++;
    return ok(jieqiSnapshot({ gameId: "jieqi-2" }));
  };

  const cancelled = game.newSession({ variant: "jieqi", play_mode: "duel" });
  await Promise.resolve();
  assert.equal(calls, 0);
  await manual.resolveDiscard("cancel");
  assert.equal(await cancelled, false);
  assert.equal(calls, 0);

  const accepted = game.newSession({ variant: "jieqi", play_mode: "duel" });
  await Promise.resolve();
  await manual.resolveDiscard("discard");
  assert.equal(await accepted, true);
  assert.equal(calls, 1);
  assert.equal(manual.dirty, false);
});
