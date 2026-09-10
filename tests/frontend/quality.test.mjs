import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mockIPC, mockWindows, clearMocks } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { after, afterEach, before, beforeEach, test } from "node:test";
import { createPinia, disposePinia, setActivePinia } from "pinia";
import { createRenderer, nextTick } from "vue";
import { createServer } from "vite";

let server;
let commands;
let originalCommands;
let useGameStore;
let useManualStore;
let useBookStore;
let parseBookEvaluation;
let useAppLifecycle;
let useEngineStore;
let defaultPreferences;
let pinia;
let initialFen;
let closeWindow;
let windowCommands;
let EngineLogPanel;

const copy = (value) => JSON.parse(JSON.stringify(value));
const ok = (data) => ({ status: "ok", data });

before(async () => {
  // 加载生产模块；仅替换桌面 IPC，棋规合法性由 Rust 测试负责
  server = await createServer({
    server: { middlewareMode: true, watch: null, ws: false },
    appType: "custom",
  });
  ({ commands } = await server.ssrLoadModule("/src/lib/ipc.ts"));
  originalCommands = { ...commands };
  ({ closeWindow } = await server.ssrLoadModule("/src/lib/window.ts"));
  ({ useGameStore } = await server.ssrLoadModule("/src/stores/game.ts"));
  ({ useManualStore } = await server.ssrLoadModule("/src/stores/manual.ts"));
  ({ useBookStore } = await server.ssrLoadModule("/src/stores/book.ts"));
  ({ parseBookEvaluation } = await server.ssrLoadModule("/src/lib/book-evaluation.ts"));
  ({ useAppLifecycle } = await server.ssrLoadModule("/src/composables/useAppLifecycle.ts"));
  ({ useEngineStore } = await server.ssrLoadModule("/src/stores/engine.ts"));
  ({ DEFAULT_PREFERENCES: defaultPreferences } = await server.ssrLoadModule("/src/lib/preferences.ts"));
  ({ INITIAL_FEN: initialFen } = await server.ssrLoadModule("/src/lib/chess.ts"));
  EngineLogPanel = (await server.ssrLoadModule("/src/components/EngineLogPanel.vue")).default;
  if (!EngineLogPanel.render) EngineLogPanel.render = () => null;
});

beforeEach(() => {
  Object.assign(commands, originalCommands);
  globalThis.window = { crypto: webcrypto, setTimeout, confirm: () => false };
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true, value: { getItem: () => null, setItem: () => {} },
  });
  windowCommands = [];
  mockWindows("main");
  mockIPC(async (command) => {
    windowCommands.push(command);
    if (command === "plugin:window|close") await emit("tauri://close-requested");
  }, { shouldMockEvents: true });
  // SDK mock 使用 id，真实事件取消订阅传入 eventId
  const invoke = window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke = (command, args, options) => invoke(
    command,
    command === "plugin:event|unlisten" ? { ...args, id: args.eventId } : args,
    options,
  );
  pinia = createPinia();
  setActivePinia(pinia);
});

afterEach(async () => {
  disposePinia(pinia);
  await flushLifecycle();
  clearMocks();
  delete globalThis.localStorage;
});
after(async () => server?.close());

function snapshot(moves, currentPly = moves.length) {
  const history = moves.map((iccs, index) => ({
    ply: index + 1,
    iccs,
    notation: iccs,
    mover: index % 2 === 0 ? "red" : "black",
    is_capture: false,
    is_check: false,
    fen: initialFen,
  }));
  return {
    fen: initialFen,
    start_fen: initialFen,
    current_fen: initialFen,
    current_ply: currentPly,
    history,
    result: "ongoing",
    red_to_move: currentPly % 2 === 0,
    in_check: false,
    can_undo: currentPly > 0,
    can_redo: currentPly < history.length,
    rule_profile: "china2020",
    repetition_count: 1,
    repetition_explanation: null,
    rule_status: "ongoing",
    rule_explanation: null,
  };
}

async function prepareGame(moves, currentPly = moves.length) {
  commands.gameResult = async () => snapshot(moves);
  const game = useGameStore();
  await game.refresh();
  const manual = useManualStore();
  for (const ply of game.history) {
    manual.recordMove(game.startFen, ply.iccs, ply.notation, ply.ply);
  }
  if (currentPly < moves.length) {
    commands.jumpTo = async (ply) => snapshot(moves, ply);
    await game.jumpTo(currentPly);
    await nextTick();
  }
  manual.dirty = false;
  return { game, manual };
}

function nodeIccs(node) {
  const square = ({ row, col }) => `${String.fromCharCode(97 + col)}${9 - row}`;
  return `${square(node.mv.from)}${square(node.mv.to)}`;
}

test("应用 PV 后保存完整棋谱，并保留原备注", async () => {
  const { game, manual } = await prepareGame(["h2e2"]);
  manual.updateComment(manual.currentNode.id, "保留备注");
  manual.dirty = false;
  manual.generatedPgn = "旧导出文本";
  commands.applyMoveLine = async (request) => {
    assert.deepEqual(request, { expected_fen: game.currentFen, moves: ["h9g7"] });
    return ok(snapshot(["h2e2", "h9g7"]));
  };
  await game.applyPreviewPrefix(["h9g7"]);
  assert.deepEqual(manual.currentPath.map(nodeIccs), ["h2e2", "h9g7"]);
  assert.equal(manual.currentPath[0].comment, "保留备注");
  assert.equal(manual.dirty, true);
  assert.equal(manual.generatedPgn, "");
  let saved;
  commands.manualSave = async (_path, value) => {
    saved = copy(value);
    return ok(null);
  };
  await manual.save("game.pgn");
  assert.equal(nodeIccs(saved.root.children[0].children[0]), "h9g7");
  assert.equal(manual.dirty, false);
});

test("历史位置应用 PV 创建分支，不删除原主线或备注", async () => {
  const { game, manual } = await prepareGame(["h2e2", "h9g7", "h0g2"], 1);
  manual.updateComment(manual.manual.root.children[0].children[0].id, "原主线备注");
  const original = copy(manual.manual.root.children[0].children[0]);
  commands.applyMoveLine = async () => ok(snapshot(["h2e2", "b9c7"]));
  await game.applyPreviewPrefix(["b9c7"]);
  const children = manual.manual.root.children[0].children;
  assert.equal(children.length, 2);
  assert.deepEqual(copy(children[0]), original);
  assert.equal(nodeIccs(children[1]), "b9c7");
  assert.deepEqual(manual.currentPath.map(nodeIccs), ["h2e2", "b9c7"]);
});

test("棋谱为空时应用 PV 包含此前对局历史", async () => {
  const { game, manual } = await prepareGame(["h2e2"]);
  manual.clear();
  commands.applyMoveLine = async () => ok(snapshot(["h2e2", "h9g7", "h0g2"]));
  await game.applyPreviewPrefix(["h9g7", "h0g2"]);
  assert.deepEqual(manual.currentPath.map(nodeIccs), ["h2e2", "h9g7", "h0g2"]);
  assert.equal(manual.dirty, true);
});

test("应用已有 PV 复用节点与备注，不标记无内容变化的棋谱", async () => {
  const { game, manual } = await prepareGame(["h2e2", "h9g7"], 1);
  const previous = copy(manual.manual);
  commands.applyMoveLine = async () => ok(snapshot(["h2e2", "h9g7"]));
  await game.applyPreviewPrefix(["h9g7"]);
  assert.deepEqual(copy(manual.manual), previous);
  assert.deepEqual(manual.currentPath.map(nodeIccs), ["h2e2", "h9g7"]);
  assert.equal(manual.dirty, false);
});

test("后端拒绝 PV 时对局与棋谱均保持不变", async () => {
  const { game, manual } = await prepareGame(["h2e2"]);
  const previous = copy(manual.manual);
  commands.applyMoveLine = async () => ({ status: "error", error: "PV 第 2 步不合法" });
  await assert.rejects(game.applyPreviewPrefix(["h9g7", "h9h9"]), /不合法/);
  assert.equal(game.currentPly, 1);
  assert.deepEqual(copy(manual.manual), previous);
  assert.equal(manual.dirty, false);
});

test("残局备注区分胜和负，未知备注不推断为胜棋", () => {
  for (const [note, expected] of [["W (M-15)", "必胜"], ["D (draw)", "官和"], ["L (M-15)", "必败"], ["未知结果", "未知结果"], [null, "-"]]) {
    assert.equal(parseBookEvaluation(note).label, expected, note);
  }
});

test("普通云库评价和分支说明保持原有含义", () => {
  for (const [mark, label] of [["!", "绝佳"], ["*", "可行"], ["?", "劣手"]]) {
    const value = parseBookEvaluation(`${mark} (44-04)`);
    assert.equal(value.label, label);
    assert.equal(value.branchesText, "44已知 / 4推荐");
  }
});

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

function prepareBookQueries() {
  const requests = new Map();
  commands.bookQuery = (fen) => {
    const request = deferred();
    requests.set(fen, request);
    return request.promise;
  };
  return { book: useBookStore(), requests };
}

test("旧开局库响应晚到时不能覆盖最新候选", async () => {
  const { book, requests } = prepareBookQueries();
  const old = book.query("旧局面");
  const current = book.query("新局面");
  requests.get("新局面").resolve(ok([{ iccs: "h9g7" }]));
  await current;
  requests.get("旧局面").resolve(ok([{ iccs: "h2e2" }]));
  await old;
  assert.deepEqual(copy(book.moves), [{ iccs: "h9g7" }]);
  assert.equal(book.loading, false);
});

test("新查询立即移除旧候选，旧响应不能提前结束 loading", async () => {
  const { book, requests } = prepareBookQueries();
  book.moves = [{ iccs: "h2e2" }];
  const old = book.query("旧局面");
  const current = book.query("新局面");
  const duringRequest = copy(book.moves);
  requests.get("旧局面").resolve(ok([{ iccs: "h2e2" }]));
  await old;
  const loadingAfterOld = book.loading;
  requests.get("新局面").resolve(ok([]));
  await current;
  assert.deepEqual(duringRequest, []);
  assert.equal(loadingAfterOld, true);
});

test("清空查询使在途响应失效", async () => {
  const { book, requests } = prepareBookQueries();
  const pending = book.query("局面");
  book.clearQuery();
  const loadingAfterClear = book.loading;
  requests.get("局面").resolve(ok([{ iccs: "h2e2" }]));
  await pending;
  assert.equal(loadingAfterClear, false);
  assert.deepEqual(copy(book.moves), []);
});

test("最新查询失败后不显示旧局面结果，错误仍可由调用方处理", async () => {
  const { book, requests } = prepareBookQueries();
  const old = book.query("旧局面");
  const current = book.query("新局面");
  const rejected = assert.rejects(current, /查询失败/);
  requests.get("新局面").reject(new Error("查询失败"));
  await rejected;
  requests.get("旧局面").resolve(ok([{ iccs: "h2e2" }]));
  await old;
  assert.deepEqual(copy(book.moves), []);
  assert.equal(book.loading, false);
});

async function flushLifecycle() {
  // 生命周期的所有 IPC 都由测试控制，只需排空 Promise 与 Vue 调度队列
  for (let index = 0; index < 10; index++) {
    await new Promise(setImmediate);
  }
}

async function mountLifecycle(t, ruleProfile) {
  const storageDescriptor = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: { getItem: () => null, setItem: () => {} },
  });
  commands.configGetLocation = async () => ok({ exists: true, isPortable: false, filePath: "/config.json" });
  commands.configLoad = async () => ok({
    ...copy(defaultPreferences),
    defaultRuleProfile: ruleProfile,
    engineProfiles: [{ id: "test", path: "/fake-engine", protocol: "uci" }],
    activeEngineId: "test",
  });
  commands.configSave = async () => ok(null);
  commands.getInitialBoard = async () => initialFen;
  commands.gameResult = async () => snapshot([]);
  commands.bookSetCloudEnabled = async () => ok(null);
  commands.bookSetCloudMode = async () => ok(null);
  const engine = useEngineStore();
  const starts = [];
  engine.start = async () => { starts.push(useGameStore().ruleProfile); };

  // 自定义 renderer 执行真实 onMounted/watch，不依赖浏览器 DOM
  const renderer = createRenderer({
    createElement: () => ({}), createText: () => ({}), createComment: () => ({}),
    insert: () => {}, remove: () => {}, setText: () => {}, setElementText: () => {},
    parentNode: () => null, nextSibling: () => null, patchProp: () => {},
  });
  const app = renderer.createApp({
    setup() { useAppLifecycle(); return () => null; },
  });
  app.use(pinia);
  app.mount({});
  t.after(async () => {
    app.unmount();
    // 等待实际偏好防抖落盘完成，避免跨用例泄漏计时器和 IPC
    await new Promise((resolve) => setTimeout(resolve, 180));
    if (storageDescriptor) Object.defineProperty(globalThis, "localStorage", storageDescriptor);
    else delete globalThis.localStorage;
  });
  await flushLifecycle();
  return { starts, engine, game: useGameStore() };
}

test("亚洲规则恢复完成前不得启动引擎", async (t) => {
  const ruleResponse = deferred();
  const requested = [];
  commands.setRuleProfile = async (profile) => {
    requested.push(profile);
    return ruleResponse.promise;
  };
  const { starts, game } = await mountLifecycle(t, "asian2017");
  const startsBeforeResponse = [...starts];
  ruleResponse.resolve({ ...snapshot([]), rule_profile: "asian2017" });
  await flushLifecycle();
  assert.deepEqual(requested, ["asian2017"]);
  assert.deepEqual(startsBeforeResponse, []);
  assert.equal(game.ruleProfile, "asian2017");
  assert.deepEqual(starts, ["asian2017"]);
});

test("默认中国规则通过同一初始化路径应用", async (t) => {
  const requested = [];
  commands.setRuleProfile = async (profile) => {
    requested.push(profile);
    return { ...snapshot([]), rule_profile: profile };
  };
  const { starts } = await mountLifecycle(t, "china2020");
  assert.deepEqual(requested, ["china2020"]);
  assert.deepEqual(starts, ["china2020"]);
});

test("规则恢复失败时错误可见且引擎不启动", async (t) => {
  commands.setRuleProfile = async () => { throw new Error("规则恢复失败"); };
  const { starts, engine } = await mountLifecycle(t, "asian2017");
  assert.deepEqual(starts, []);
  assert.match(engine.lastError, /规则恢复失败/);
});

async function makeNextMove(game, moves) {
  const iccs = moves[moves.length - 1];
  commands.makeMove = async () => ok({
    legal: true, iccs, chinese_notation: iccs, check: false, game_over: false,
  });
  commands.gameResult = async () => snapshot(moves);
  await game.makeMove(iccs);
}

test("清空棋谱后继续走子从权威历史恢复完整路径", async () => {
  const { game, manual } = await prepareGame(["h2e2"]);
  manual.clear();
  await makeNextMove(game, ["h2e2", "h9g7"]);
  assert.deepEqual(manual.currentPath.map(nodeIccs), ["h2e2", "h9g7"]);
  assert.equal(manual.manual.start_fen, game.startFen);
  assert.equal(manual.dirty, true);
  let saved;
  commands.manualSave = async (_path, value) => { saved = copy(value); return ok(null); };
  await manual.save("game.pgn");
  assert.equal(nodeIccs(saved.root.children[0]), "h2e2");
  assert.equal(nodeIccs(saved.root.children[0].children[0]), "h9g7");
});

test("历史位置继续走子保留原主线与备注", async () => {
  const { game, manual } = await prepareGame(["h2e2", "h9g7"], 1);
  const originalNode = manual.manual.root.children[0].children[0];
  manual.updateComment(originalNode.id, "原分支备注");
  await makeNextMove(game, ["h2e2", "b9c7"]);
  const children = manual.manual.root.children[0].children;
  assert.deepEqual(children.map(nodeIccs), ["h9g7", "b9c7"]);
  assert.equal(children[0].comment, "原分支备注");
  assert.deepEqual(manual.currentPath.map(nodeIccs), ["h2e2", "b9c7"]);
});

for (const [method, command, path] of [
  ["save", "manualSave", "game.pgn"],
  ["saveXqf", "manualSaveXqf", "game.xqf"],
]) {
  test(`${method} 只清除已保存版本的修改标记`, async () => {
    const { manual } = await prepareGame(["h2e2"]);
    manual.updateComment(manual.currentNode.id, "保存版本");
    const response = deferred();
    let saved;
    commands[command] = async (_path, value) => { saved = copy(value); return response.promise; };
    const saving = manual[method](path);
    manual.updateComment(manual.currentNode.id, "新编辑");
    response.resolve(ok(null));
    await saving;
    assert.equal(saved.root.children[0].comment, "保存版本");
    assert.equal(manual.currentNode.comment, "新编辑");
    assert.equal(manual.dirty, true);
    commands[command] = async () => ok(null);
    await manual[method](path);
    assert.equal(manual.dirty, false);
  });

  test(`${method} 旧响应不影响替换后的棋谱，失败保留待保存状态`, async () => {
    const { manual } = await prepareGame(["h2e2"]);
    const response = deferred();
    commands[command] = async () => response.promise;
    const saving = manual[method](path);
    manual.clear();
    manual.updateComment(0, "新棋谱备注");
    response.resolve(ok(null));
    await saving;
    assert.equal(manual.dirty, true);
    commands[command] = async () => ({ status: "error", error: "磁盘写入失败" });
    await manual[method](path);
    assert.equal(manual.dirty, true);
    assert.equal(manual.error, "磁盘写入失败");
  });
}

for (const result of ["blackwin", "draw"]) {
  test(`同一局面终局 ${result} 停止分析并可在恢复对局后重新分析`, async (t) => {
    commands.setRuleProfile = async () => snapshot([]);
    const { game, engine } = await mountLifecycle(t, "china2020");
    let analyses = 0;
    let stops = 0;
    commands.engineAnalyze = async () => { analyses++; return ok({ started: true, constraint_status: "not_applied" }); };
    commands.engineMoveNow = async () => { stops++; return ok(null); };
    engine.running = true;
    engine.analysisEnabled = true;
    await flushLifecycle();
    assert.equal(engine.analyzing, true);
    commands.resign = async () => ok({ ...snapshot([]), result });
    await game.resign("red");
    await flushLifecycle();
    assert.equal(stops, 1);
    assert.equal(engine.analyzing, false);
    assert.equal(engine.running, true);
    const previousAnalyses = analyses;
    commands.gameResult = async () => snapshot([]);
    await game.refresh();
    await flushLifecycle();
    assert.equal(analyses, previousAnalyses + 1);
    assert.equal(engine.analyzing, true);
  });
}

test("关闭软件直接关闭，不需要未保存棋谱提示", async () => {
  const { manual } = await prepareGame(["h2e2"]);
  manual.updateComment(manual.currentNode.id, "未保存备注");
  assert.equal(manual.dirty, true);
  window.confirm = () => {
    assert.fail("不应弹出未保存棋谱确认窗口");
  };
  await closeWindow();
  await flushLifecycle();
  assert.equal(windowCommands.includes("plugin:window|close"), true);
});

test("棋谱 confirmDiscard 直接返回 true，由用户自行保存", () => {
  const manual = useManualStore();
  window.confirm = () => {
    assert.fail("不应弹出未保存棋谱确认窗口");
  };
  assert.equal(manual.confirmDiscard(), true);
});

function mountEngineLogPanel() {
  const renderer = createRenderer({
    createElement: () => ({}), createText: () => ({}), createComment: () => ({}),
    insert: () => {}, remove: () => {}, setText: () => {}, setElementText: () => {},
    parentNode: () => null, nextSibling: () => null, patchProp: () => {},
  });
  const app = renderer.createApp(EngineLogPanel);
  app.provide(Symbol.for("v-scx"), { modules: new Set() });
  app.use(pinia);
  const vm = app.mount({});
  return { app, vm };
}

test("协议日志面板展开时订阅并拉取快照，收起时注销事件监听", async () => {
  commands.engineProtocolLog = async () => ok([
    {
      run_id: "run-1",
      analysis_session_id: "session-1",
      timestamp_ms: 1000,
      sequence: 1,
      stream: "stdout",
      raw: "uciok\n",
    },
  ]);

  const { app, vm } = mountEngineLogPanel();
  await flushLifecycle();

  assert.equal(vm.open, false);
  assert.equal(vm.lines.length, 0);

  // 面板关闭时，发送事件不应被接收
  await emit("engine://protocol", {
    run_id: "run-1",
    analysis_session_id: "session-1",
    timestamp_ms: 1010,
    sequence: 2,
    stream: "stdout",
    raw: "ignored-while-closed\n",
  });
  await flushLifecycle();
  assert.equal(vm.lines.length, 0);

  // 展开面板：开始订阅并加载快照
  await vm.toggle();
  await flushLifecycle();

  assert.equal(vm.open, true);
  assert.equal(vm.lines.length, 1);
  assert.equal(vm.lines[0].raw, "uciok\n");

  // 面板展开时发送新事件，能够正常接收
  await emit("engine://protocol", {
    run_id: "run-1",
    analysis_session_id: "session-1",
    timestamp_ms: 1050,
    sequence: 3,
    stream: "stdout",
    raw: "readyok\n",
  });
  await flushLifecycle();
  assert.equal(vm.lines.length, 2);
  assert.equal(vm.lines[1].raw, "readyok\n");

  // 收起面板：立即注销事件监听
  await vm.toggle();
  await flushLifecycle();
  assert.equal(vm.open, false);

  // 面板收起后，新发出的事件不再被处理
  await emit("engine://protocol", {
    run_id: "run-1",
    analysis_session_id: "session-1",
    timestamp_ms: 1100,
    sequence: 4,
    stream: "stdout",
    raw: "info depth 1\n",
  });
  await flushLifecycle();
  assert.equal(vm.lines.length, 2);

  // 再次展开：重新建立订阅并接收新事件
  commands.engineProtocolLog = async () => ok(vm.lines);
  await vm.toggle();
  await flushLifecycle();
  assert.equal(vm.open, true);

  await emit("engine://protocol", {
    run_id: "run-1",
    analysis_session_id: "session-1",
    timestamp_ms: 1150,
    sequence: 5,
    stream: "stdout",
    raw: "bestmove h2e2\n",
  });
  await flushLifecycle();
  assert.equal(vm.lines.length, 3);
  assert.equal(vm.lines[2].raw, "bestmove h2e2\n");

  // 卸载面板
  app.unmount();
  await flushLifecycle();

  // 卸载后发送事件也不再接收
  await emit("engine://protocol", {
    run_id: "run-1",
    analysis_session_id: "session-1",
    timestamp_ms: 1200,
    sequence: 6,
    stream: "stdout",
    raw: "post-unmount\n",
  });
  await flushLifecycle();
  assert.equal(vm.lines.length, 3);
});

test("协议日志面板展开中途收起能及时清理监听器", async () => {
  commands.engineProtocolLog = async () => ok([]);
  const { app, vm } = mountEngineLogPanel();
  await flushLifecycle();

  // 连续点击展开又收起
  const p1 = vm.toggle();
  const p2 = vm.toggle();
  await Promise.all([p1, p2]);
  await flushLifecycle();

  assert.equal(vm.open, false);

  // 此时监听器应已被清理，发送事件不被接收
  await emit("engine://protocol", {
    run_id: "run-1",
    analysis_session_id: "session-1",
    timestamp_ms: 1000,
    sequence: 1,
    stream: "stdout",
    raw: "should-not-receive\n",
  });
  await flushLifecycle();
  assert.equal(vm.lines.length, 0);

  app.unmount();
  await flushLifecycle();
});
