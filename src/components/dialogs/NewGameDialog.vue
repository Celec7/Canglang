<script setup lang="ts">
import { ref, watch } from "vue";
import type { JieqiPlayMode, RuleProfile } from "@/bindings";
import {
  Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader,
  DialogTitle, Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue,
} from "@/components/ui";
import { useGameStore } from "@/stores/game";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ (event: "update:open", value: boolean): void }>();
const game = useGameStore();
const variant = ref<"xiangqi" | "jieqi">("xiangqi");
const playMode = ref<JieqiPlayMode>("duel");
const ruleProfile = ref<RuleProfile>("china2020");
const starting = ref(false);
const error = ref<string | null>(null);

watch(() => props.open, (open) => {
  if (!open) return;
  variant.value = game.variant ?? "xiangqi";
  playMode.value = game.playMode ?? "duel";
  ruleProfile.value = game.ruleProfile ?? "china2020";
  error.value = null;
});

async function start() {
  if (starting.value) return;
  starting.value = true;
  error.value = null;
  try {
    const options = variant.value === "jieqi"
      ? { variant: "jieqi" as const, play_mode: playMode.value }
      : { variant: "xiangqi" as const, fen: null, rule_profile: ruleProfile.value };
    if (await game.newSession(options)) emit("update:open", false);
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  } finally {
    starting.value = false;
  }
}
</script>

<template>
  <Dialog :open="props.open" @update:open="emit('update:open', $event)">
    <DialogContent class="max-w-md">
      <DialogHeader>
        <DialogTitle>开始新对局</DialogTitle>
        <DialogDescription>选择棋种和对局方式。揭棋建立在象棋走法之上，暗子首次移动后公开身份。</DialogDescription>
      </DialogHeader>

      <div class="flex flex-col gap-4 py-2">
        <label class="flex flex-col gap-1.5 text-xs font-medium">
          棋种
          <Select v-model="variant">
            <SelectTrigger aria-label="棋种"><SelectValue /></SelectTrigger>
            <SelectContent><SelectGroup>
              <SelectItem value="xiangqi">中国象棋</SelectItem>
              <SelectItem value="jieqi">揭棋</SelectItem>
            </SelectGroup></SelectContent>
          </Select>
        </label>

        <label v-if="variant === 'jieqi'" class="flex flex-col gap-1.5 text-xs font-medium">
          对局方式
          <Select v-model="playMode">
            <SelectTrigger aria-label="揭棋对局方式"><SelectValue /></SelectTrigger>
            <SelectContent><SelectGroup>
              <SelectItem value="duel">对弈：不允许悔棋和改走</SelectItem>
              <SelectItem value="training">训练：可撤销、重做和改走</SelectItem>
            </SelectGroup></SelectContent>
          </Select>
        </label>

        <label v-else class="flex flex-col gap-1.5 text-xs font-medium">
          规则档案
          <Select v-model="ruleProfile">
            <SelectTrigger aria-label="象棋规则档案"><SelectValue /></SelectTrigger>
            <SelectContent><SelectGroup>
              <SelectItem value="china2020">中国规则 2020</SelectItem>
              <SelectItem value="asian2017">亚洲/世界规则 2017</SelectItem>
            </SelectGroup></SelectContent>
          </Select>
        </label>

        <p v-if="variant === 'jieqi'" class="rounded-md bg-muted px-3 py-2 text-xs leading-relaxed text-muted-foreground">
          当前采用本地休闲规则：暗士、暗象先按原位置角色行走；揭开后按真实身份行走。暂不提供引擎分析和开局库。
        </p>
        <p v-if="error" role="alert" class="text-xs text-destructive">{{ error }}</p>
      </div>

      <DialogFooter>
        <Button variant="outline" :disabled="starting" @click="emit('update:open', false)">取消</Button>
        <Button :disabled="starting" @click="start">{{ starting ? "正在开始…" : "开始对局" }}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
