<script setup lang="ts">
import { computed } from "vue";
import { Badge, Separator } from "@/components/ui";
import { jieqiPieceName } from "@/lib/chess";
import { useGameStore } from "@/stores/game";

const game = useGameStore();
const position = computed(() => game.position?.variant === "jieqi" ? game.position.position : null);
const hidden = computed(() => position.value?.pieces.filter((piece) => piece.state === "hidden").length ?? 0);
const revealed = computed(() => position.value?.pieces.filter((piece) => piece.state === "revealed").length ?? 0);
const captured = computed(() => game.jieqiHistory.filter((ply) => ply.captured.type !== "none"));
const latestReveals = computed(() => game.jieqiHistory.filter((ply) => ply.revealed).slice(-6).reverse());
</script>

<template>
  <section class="flex h-full min-h-0 flex-col rounded-xl border bg-card shadow-sm">
    <header class="flex min-h-11 items-center justify-between gap-2 px-3">
      <div>
        <h3 class="text-sm font-semibold">揭棋公开信息</h3>
        <p class="text-xs text-muted-foreground">只显示双方都已知的内容</p>
      </div>
      <Badge variant="secondary">{{ game.playMode === "duel" ? "对弈" : "训练" }}</Badge>
    </header>
    <Separator />
    <div class="flex flex-col gap-4 p-3 text-sm">
      <div class="grid grid-cols-3 gap-2 text-center">
        <div class="rounded-md bg-muted p-2"><strong class="block text-lg">{{ hidden }}</strong><span class="text-xs text-muted-foreground">未揭暗子</span></div>
        <div class="rounded-md bg-muted p-2"><strong class="block text-lg">{{ revealed }}</strong><span class="text-xs text-muted-foreground">盘上明子</span></div>
        <div class="rounded-md bg-muted p-2"><strong class="block text-lg">{{ captured.length }}</strong><span class="text-xs text-muted-foreground">公开吃子</span></div>
      </div>
      <div>
        <h4 class="mb-2 text-xs font-semibold">最近揭子</h4>
        <ul v-if="latestReveals.length" class="flex flex-col gap-1.5 text-xs">
          <li v-for="ply in latestReveals" :key="ply.ply" class="flex justify-between gap-2">
            <span>第 {{ ply.ply }} 手 · {{ ply.notation }}</span>
            <Badge variant="outline">{{ jieqiPieceName(ply.revealed, ply.mover) }}</Badge>
          </li>
        </ul>
        <p v-else class="text-xs text-muted-foreground">尚未揭开暗子</p>
      </div>
      <p class="rounded-md bg-muted px-3 py-2 text-xs leading-relaxed text-muted-foreground">
        揭棋沿用象棋的车、马、炮、兵和王安全规则；区别只在暗子首步角色、揭子后的真实角色及信息公开方式。
      </p>
    </div>
  </section>
</template>
