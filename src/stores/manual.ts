import { computed, ref, watch } from "vue";
import { defineStore } from "pinia";
import type {
  ChessManual, JieqiDocumentKind, JieqiDocumentMetadata, ManualNode, PlyRecord,
} from "@/bindings";
import { coordsToIccs, iccsToCoords, INITIAL_FEN } from "@/lib/chess";
import { commands, unwrap, unwrapSession } from "@/lib/ipc";
import { useGameStore } from "@/stores/game";

function syncFromGame(startFen: string, history: PlyRecord[]): ChessManual {
  const root: ManualNode = {
    id: 0,
    mv: null,
    chinese_notation: "开始局面",
    comment: null,
    score: null,
    children: [],
  };

  let current = root;
  for (const ply of history) {
    const coords = iccsToCoords(ply.iccs);
    const child: ManualNode = {
      id: ply.ply,
      mv: coords
        ? {
            from: { row: coords.from[0], col: coords.from[1] },
            to: { row: coords.to[0], col: coords.to[1] },
          }
        : null,
      chinese_notation: ply.notation,
      comment: null,
      score: null,
      children: [],
    };
    current.children.push(child);
    current = child;
  }

  return {
    title: "对局记录",
    date: new Date().toISOString().slice(0, 10),
    red_player: null,
    black_player: null,
    event_name: null,
    start_fen: startFen || INITIAL_FEN,
    root,
  };
}

function nodeMove(node: ManualNode): string | null {
  return node.mv ? coordsToIccs([node.mv.from.row, node.mv.from.col], [node.mv.to.row, node.mv.to.col]) : null;
}

function lastNode(nodes: ManualNode[]): ManualNode | undefined {
  return nodes[nodes.length - 1];
}

function maxNodeId(node: ManualNode): number {
  return Math.max(node.id, ...node.children.map((child) => maxNodeId(child)));
}

function appendMainline(node: ManualNode, result: ManualNode[]) {
  let current: ManualNode | undefined = node;
  while (current) {
    result.push(current);
    current = current.children[0];
  }
}

function findPath(root: ManualNode, targetId: number, path: ManualNode[] = []): ManualNode[] | null {
  const nextPath = [...path, root];
  if (root.id === targetId) return nextPath;
  for (const child of root.children) {
    const found = findPath(child, targetId, nextPath);
    if (found) return found;
  }
  return null;
}

function createManual(startFen: string): ChessManual {
  return {
    title: "未命名棋谱",
    date: null,
    red_player: null,
    black_player: null,
    event_name: null,
    start_fen: startFen || INITIAL_FEN,
    root: {
      id: 0,
      mv: null,
      chinese_notation: "开始局面",
      comment: null,
      score: null,
      children: [],
    },
  };
}

function emptyJieqiMetadata(): JieqiDocumentMetadata {
  return {
    title: "未命名揭棋",
    date: new Date().toISOString().slice(0, 10),
    red_player: "",
    black_player: "",
    event_name: "",
  };
}

export const useManualStore = defineStore("manual", () => {
  const game = useGameStore();
  const manual = ref<ChessManual | null>(null);
  const documentKind = ref<"xiangqi" | JieqiDocumentKind>("xiangqi");
  const documentPath = ref<string | null>(null);
  const xiangqiFormat = ref<"pgn" | "xqf">("pgn");
  const jieqiMetadata = ref<JieqiDocumentMetadata>(emptyJieqiMetadata());
  const jieqiAnnotations = ref<Record<number, string>>({});
  const editRevision = ref(0);
  const generatedPgn = ref("");
  const loading = ref(false);
  const error = ref<string | null>(null);
  const dirty = ref(false);
  const activeLineIds = ref<number[]>([]);
  const cursorDepth = ref(0);
  const nextNodeId = ref(1);
  const discardPromptOpen = ref(false);
  const resolvingDiscard = ref(false);
  let pendingDiscard: Promise<boolean> | null = null;
  let resolvePendingDiscard: ((proceed: boolean) => void) | null = null;
  let savedGameId: string | null = null;
  let savedContentRevision: string | null = null;

  function currentXiangqiRecord(): { startFen: string; history: PlyRecord[] } | null {
    if (!game.startFen || game.variant !== "xiangqi") return null;
    return { startFen: game.startFen, history: game.xiangqiHistory };
  }

  // 所有公开文档编辑共享一个修订号，保存响应不能清除在途产生的新编辑
  watch(manual, () => { editRevision.value++; }, { deep: true, flush: "sync" });
  watch([jieqiMetadata, jieqiAnnotations], () => {
    editRevision.value++;
    if (game.variant === "jieqi") dirty.value = true;
  }, { deep: true, flush: "sync" });

  watch(
    () => [game.gameId, game.contentRevision, game.snapshot?.head_ply, game.variant] as const,
    ([gameId, currentContentRevision, headPly, variant], previous) => {
      if (variant !== "jieqi" || gameId !== savedGameId) return;
      if (currentContentRevision !== savedContentRevision) dirty.value = true;
      const previousHead = previous?.[2];
      if (typeof previousHead === "number" && typeof headPly === "number" && headPly < previousHead) {
        const retained = Object.fromEntries(
          Object.entries(jieqiAnnotations.value).filter(([ply]) => Number(ply) <= headPly),
        );
        if (Object.keys(retained).length !== Object.keys(jieqiAnnotations.value).length) {
          jieqiAnnotations.value = retained;
        }
      }
    },
    { flush: "sync" },
  );

  const currentPath = computed(() => {
    if (!manual.value) return [];
    return activeLineIds.value.slice(0, cursorDepth.value)
      .map((id) => lastNode(findPath(manual.value!.root, id) ?? []) ?? null)
      .filter((node): node is ManualNode => !!node);
  });

  const activeLine = computed(() => {
    if (!manual.value) return [];
    return activeLineIds.value
      .map((id) => lastNode(findPath(manual.value!.root, id) ?? []) ?? null)
      .filter((node): node is ManualNode => !!node);
  });

  const mainlineNodes = computed(() => {
    if (!manual.value) return [];
    const nodes: ManualNode[] = [];
    appendMainline(manual.value.root, nodes);
    return nodes.filter((node) => node.mv);
  });

  const currentNode = computed(() => {
    if (!manual.value) return null;
    return lastNode(currentPath.value) ?? manual.value.root;
  });

  const nextBranches = computed(() => currentNode.value?.children ?? []);

  function resetPath() {
    activeLineIds.value = [];
    cursorDepth.value = 0;
    nextNodeId.value = manual.value ? maxNodeId(manual.value.root) + 1 : 1;
  }

  function setPath(nodes: ManualNode[]) {
    activeLineIds.value = nodes.filter((node) => node.mv).map((node) => node.id);
    cursorDepth.value = activeLineIds.value.length;
    nextNodeId.value = manual.value ? maxNodeId(manual.value.root) + 1 : 1;
  }

  function recordMove(startFen: string, iccs: string, notation: string, depth: number) {
    if (!manual.value) manual.value = createManual(startFen);
    const boundedDepth = Math.max(0, depth - 1);
    const sourcePath = activeLineIds.value;
    const parentPath = sourcePath.slice(0, boundedDepth);
    const parent = parentPath.length > 0
      ? lastNode(findPath(manual.value.root, parentPath[parentPath.length - 1]) ?? []) ?? manual.value.root
      : manual.value.root;
    let child = parent.children.find((candidate) => nodeMove(candidate) === iccs);
    if (!child) {
      const move = iccsToCoords(iccs);
      if (!move) return;
      child = {
        id: nextNodeId.value++,
        mv: {
          from: { row: move.from[0], col: move.from[1] },
          to: { row: move.to[0], col: move.to[1] },
        },
        chinese_notation: notation,
        comment: null,
        score: null,
        children: [],
      };
      parent.children.push(child);
      generatedPgn.value = "";
      dirty.value = true;
    }
    const nextPath = [...parentPath, child.id];
    activeLineIds.value = nextPath;
    cursorDepth.value = nextPath.length;
  }

  // 以权威历史定位整条当前路径，复用已有节点，保留原分支与备注
  function recordHistory(startFen: string, history: PlyRecord[]) {
    for (const ply of history) {
      recordMove(startFen, ply.iccs, ply.notation, ply.ply);
    }
  }

  function initializeForCurrentSession() {
    documentPath.value = null;
    generatedPgn.value = "";
    error.value = null;
    if (game.variant === "jieqi") {
      documentKind.value = "private_game";
      manual.value = null;
      jieqiMetadata.value = emptyJieqiMetadata();
      jieqiAnnotations.value = {};
    } else {
      documentKind.value = "xiangqi";
      manual.value = null;
      xiangqiFormat.value = "pgn";
      resetPath();
    }
    savedGameId = game.gameId;
    savedContentRevision = game.contentRevision;
    dirty.value = false;
  }

  async function load(path: string) {
    loading.value = true;
    error.value = null;
    try {
      manual.value = await unwrap(await commands.manualLoad(path.trim()));
      documentKind.value = "xiangqi";
      documentPath.value = path.trim();
      xiangqiFormat.value = path.trim().toLowerCase().endsWith(".xqf") ? "xqf" : "pgn";
      generatedPgn.value = "";
      resetPath();
      dirty.value = false;
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading.value = false;
    }
  }

  async function pickFile(action: "open" | "save" | "save_xqf" | "open_jieqi" | "save_jieqi_private" | "save_jieqi_public"): Promise<string | null> {
    return unwrap(await commands.manualPickFile(action));
  }

  async function openXiangqi(path: string): Promise<boolean> {
    const normalizedPath = path.trim();
    if (!normalizedPath) return false;
    loading.value = true;
    error.value = null;
    const previous = {
      manual: manual.value,
      kind: documentKind.value,
      path: documentPath.value,
      format: xiangqiFormat.value,
      line: [...activeLineIds.value],
      cursor: cursorDepth.value,
      nextId: nextNodeId.value,
      dirty: dirty.value,
      savedGameId,
      savedContentRevision,
    };
    try {
      const candidate = await unwrap(await commands.manualLoad(normalizedPath));
      manual.value = candidate;
      resetPath();
      const nodes: ManualNode[] = [];
      appendMainline(candidate.root, nodes);
      if (game.variant === "jieqi") {
        const reset = await game.newGame(candidate.start_fen || undefined, {
          preserveManual: true,
          skipManualGuard: true,
        });
        if (!reset) throw new Error("无法建立棋谱起始局面");
        for (const node of nodes) {
          const iccs = nodeMove(node);
          if (iccs && !(await game.replayMove(iccs)).legal) throw new Error(`走法无法应用：${iccs}`);
        }
        setPath(nodes);
      } else if (!(await applyNodes(nodes))) {
        throw new Error(error.value ?? "棋谱中包含无法应用的走法");
      }
      documentKind.value = "xiangqi";
      documentPath.value = normalizedPath;
      xiangqiFormat.value = normalizedPath.toLowerCase().endsWith(".xqf") ? "xqf" : "pgn";
      savedGameId = game.gameId;
      savedContentRevision = game.contentRevision;
      dirty.value = false;
      return true;
    } catch (cause) {
      manual.value = previous.manual;
      documentKind.value = previous.kind;
      documentPath.value = previous.path;
      xiangqiFormat.value = previous.format;
      activeLineIds.value = previous.line;
      cursorDepth.value = previous.cursor;
      nextNodeId.value = previous.nextId;
      dirty.value = previous.dirty;
      savedGameId = previous.savedGameId;
      savedContentRevision = previous.savedContentRevision;
      error.value = cause instanceof Error ? cause.message : String(cause);
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function openJieqi(path: string): Promise<boolean> {
    const normalizedPath = path.trim();
    if (!normalizedPath) return false;
    loading.value = true;
    error.value = null;
    try {
      const opened = await game.openJieqiDocument(normalizedPath);
      documentKind.value = opened.document.kind;
      documentPath.value = normalizedPath;
      manual.value = null;
      jieqiMetadata.value = { ...opened.document.metadata };
      jieqiAnnotations.value = { ...opened.document.annotations };
      savedGameId = opened.snapshot.game_id;
      savedContentRevision = opened.snapshot.content_revision;
      dirty.value = false;
      return true;
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function applyNodes(nodes: ManualNode[]) {
    if (!manual.value) return false;
    const previous = currentXiangqiRecord();
    if (!previous) return false;
    error.value = null;
    const preservedManual = manual.value;
    const previousStartFen = previous.startFen;
    const previousHistory = previous.history.map((ply) => ply.iccs);
    const previousPly = game.currentPly;
    const previousActiveLineIds = [...activeLineIds.value];
    const previousCursorDepth = cursorDepth.value;
    const previousDirty = dirty.value;
    try {
      const reset = await game.newGame(manual.value.start_fen || undefined, {
        preserveManual: true,
        skipManualGuard: true,
      });
      if (!reset) throw new Error("无法重置棋盘");
      manual.value = preservedManual;
      resetPath();
      for (const node of nodes) {
        if (!node.mv) continue;
        const iccs = nodeMove(node);
        if (!iccs || !(await game.makeMove(iccs)).legal) throw new Error(`走法无法应用：${iccs ?? "未知"}`);
      }
      setPath(nodes);
      return true;
    } catch (cause) {
      try {
        await game.newGame(previousStartFen, {
          preserveManual: true,
          skipManualGuard: true,
        });
        for (const iccs of previousHistory) {
          if (!(await game.replayMove(iccs)).legal) {
            throw new Error(`无法恢复原棋局走法：${iccs}`);
          }
        }
        await game.jumpTo(previousPly);
        activeLineIds.value = previousActiveLineIds;
        cursorDepth.value = previousCursorDepth;
        nextNodeId.value = manual.value ? maxNodeId(manual.value.root) + 1 : 1;
        dirty.value = previousDirty;
      } catch (restoreCause) {
        error.value = `${cause instanceof Error ? cause.message : String(cause)}；恢复原棋局失败：${restoreCause instanceof Error ? restoreCause.message : String(restoreCause)}`;
        return false;
      }
      error.value = cause instanceof Error ? cause.message : String(cause);
      return false;
    }
  }

  async function applyToGame(): Promise<boolean> {
    if (!manual.value) return false;
    const nodes: ManualNode[] = [];
    appendMainline(manual.value.root, nodes);
    return applyNodes(nodes);
  }

  function pathToNode(nodeId: number): ManualNode[] | null {
    return manual.value ? findPath(manual.value.root, nodeId) : null;
  }

  function selectPly(ply: number): boolean {
    if (ply < 0) return false;
    if (game.variant === "jieqi") return ply <= (game.snapshot?.head_ply ?? 0);
    const current = currentXiangqiRecord();
    if (!current) return false;
    if (!manual.value) {
      manual.value = current.history.length > 0
        ? syncFromGame(current.startFen, current.history)
        : createManual(current.startFen);
      resetPath();
    }

    const node = ply === 0
      ? manual.value.root
      : activeLine.value[ply - 1] ?? mainlineNodes.value[ply - 1];
    if (!node) return false;

    const path = findPath(manual.value.root, node.id);
    if (!path) return false;
    setPath(path);
    return true;
  }

  function annotationForPly(ply: number): string | null {
    if (game.variant === "jieqi") return jieqiAnnotations.value[ply] ?? null;
    if (!manual.value) return null;
    const node = ply === 0 ? manual.value.root : activeLine.value[ply - 1] ?? mainlineNodes.value[ply - 1];
    return node?.comment ?? null;
  }

  function updateJieqiComment(ply: number, text: string): boolean {
    if (game.variant !== "jieqi" || !game.capabilities.edit_annotations.enabled) return false;
    if (ply < 0 || ply > (game.snapshot?.head_ply ?? 0)) return false;
    const normalized = text.trim().length === 0 ? null : text;
    if ((jieqiAnnotations.value[ply] ?? null) === normalized) return true;
    const next = { ...jieqiAnnotations.value };
    if (normalized === null) delete next[ply];
    else next[ply] = normalized;
    jieqiAnnotations.value = next;
    return true;
  }

  function confirmDiscard(): Promise<boolean> {
    if (!dirty.value) return Promise.resolve(true);
    if (pendingDiscard) return pendingDiscard;
    discardPromptOpen.value = true;
    pendingDiscard = new Promise<boolean>((resolve) => {
      resolvePendingDiscard = resolve;
    });
    return pendingDiscard;
  }

  async function resolveDiscard(action: "save" | "discard" | "cancel") {
    if (!resolvePendingDiscard || resolvingDiscard.value) return;
    resolvingDiscard.value = true;
    let proceed = action === "discard";
    if (action === "save") {
      try {
        proceed = await saveNative();
      } catch (cause) {
        error.value = cause instanceof Error ? cause.message : String(cause);
        proceed = false;
      }
      if (!proceed) {
        resolvingDiscard.value = false;
        return;
      }
    }
    const resolve = resolvePendingDiscard;
    resolvePendingDiscard = null;
    pendingDiscard = null;
    discardPromptOpen.value = false;
    resolvingDiscard.value = false;
    resolve(proceed);
  }

  function updateComment(nodeId: number, text: string): boolean {
    const current = currentXiangqiRecord();
    if (!current) return false;
    let targetManual = manual.value;
    if (!targetManual) {
      const candidate = current.history.length > 0
        ? syncFromGame(current.startFen, current.history)
        : createManual(current.startFen);
      if (!findPath(candidate.root, nodeId)) return false;
      manual.value = candidate;
      resetPath();
      targetManual = candidate;
    }

    const node = lastNode(findPath(targetManual.root, nodeId) ?? []);
    if (!node) return false;

    const normalized = text.trim().length === 0 ? null : text;
    if (node.comment === normalized) return true;

    node.comment = normalized;
    generatedPgn.value = "";
    dirty.value = true;
    return true;
  }

  watch(
    () => game.currentPly,
    (depth) => {
      cursorDepth.value = Math.max(0, Math.min(depth, activeLineIds.value.length));
    }
  );

  async function exportPgn() {
    const current = currentXiangqiRecord();
    if (!current) return;
    if (!manual.value && current.history.length > 0) manual.value = syncFromGame(current.startFen, current.history);
    if (!manual.value) manual.value = createManual(current.startFen);
    error.value = null;
    try {
      generatedPgn.value = await commands.manualExportPgn(manual.value);
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function save(path: string) {
    const current = currentXiangqiRecord();
    if (!current) return;
    if (!manual.value && current.history.length > 0) manual.value = syncFromGame(current.startFen, current.history);
    if (!manual.value) return;
    error.value = null;
    const savedRevision = editRevision.value;
    try {
      await unwrap(await commands.manualSave(path.trim(), manual.value));
      if (editRevision.value === savedRevision && documentKind.value === "xiangqi") {
        documentPath.value = path.trim();
        xiangqiFormat.value = "pgn";
        savedGameId = game.gameId;
        savedContentRevision = game.contentRevision;
        dirty.value = false;
      }
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function saveXqf(path: string, version = 10) {
    const current = currentXiangqiRecord();
    if (!current) return;
    if (!manual.value && current.history.length > 0) manual.value = syncFromGame(current.startFen, current.history);
    if (!manual.value) return;
    error.value = null;
    const savedRevision = editRevision.value;
    try {
      await unwrap(await commands.manualSaveXqf(path.trim(), manual.value, version));
      if (editRevision.value === savedRevision && documentKind.value === "xiangqi") {
        documentPath.value = path.trim();
        xiangqiFormat.value = "xqf";
        savedGameId = game.gameId;
        savedContentRevision = game.contentRevision;
        dirty.value = false;
      }
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function saveJieqi(path: string, kind: JieqiDocumentKind): Promise<boolean> {
    if (game.variant !== "jieqi" || !path.trim()) return false;
    error.value = null;
    const requestedEditRevision = editRevision.value.toString();
    try {
      const receipt = await unwrapSession(await commands.jieqiDocumentSave(
        game.token(),
        path.trim(),
        kind,
        { ...jieqiMetadata.value },
        { ...jieqiAnnotations.value },
        requestedEditRevision,
      ));
      const isNative = documentKind.value === kind;
      if (
        isNative
        && receipt.game_id === game.gameId
        && receipt.content_revision === game.contentRevision
        && receipt.edit_revision === editRevision.value.toString()
        && receipt.kind === documentKind.value
      ) {
        documentPath.value = path.trim();
        savedGameId = receipt.game_id;
        savedContentRevision = receipt.content_revision;
        dirty.value = false;
      }
      return true;
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      return false;
    }
  }

  async function saveNative(): Promise<boolean> {
    if (documentKind.value === "xiangqi") {
      let path = documentPath.value;
      if (!path) path = await pickFile(xiangqiFormat.value === "xqf" ? "save_xqf" : "save");
      if (!path) return false;
      if (xiangqiFormat.value === "xqf") await saveXqf(path.endsWith(".xqf") ? path : `${path}.xqf`);
      else await save(path.endsWith(".pgn") ? path : `${path}.pgn`);
      return !dirty.value;
    }

    const kind = documentKind.value;
    let path = documentPath.value;
    if (!path) {
      path = await pickFile(kind === "private_game" ? "save_jieqi_private" : "save_jieqi_public");
    }
    if (!path) return false;
    const normalized = path.endsWith(".cjq") ? path : `${path}.cjq`;
    const succeeded = await saveJieqi(normalized, kind);
    return succeeded && !dirty.value;
  }

  function clear() {
    initializeForCurrentSession();
  }

  return {
    manual,
    documentKind,
    documentPath,
    jieqiMetadata,
    jieqiAnnotations,
    editRevision,
    generatedPgn,
    loading,
    error,
    dirty,
    hasUnsavedChanges: computed(() => dirty.value),
    discardPromptOpen,
    resolvingDiscard,
    currentPath,
    activeLine,
    mainlineNodes,
    currentNode,
    nextBranches,
    load,
    openXiangqi,
    openJieqi,
    pickFile,
    recordMove,
    recordHistory,
    applyNodes,
    applyToGame,
    pathToNode,
    selectPly,
    confirmDiscard,
    resolveDiscard,
    annotationForPly,
    updateJieqiComment,
    updateComment,
    exportPgn,
    save,
    saveXqf,
    saveJieqi,
    saveNative,
    clear,
  };
});
