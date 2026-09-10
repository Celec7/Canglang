<script setup lang="ts">
import {
  Button,
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui";
import { ArrowLeftRight, Edit3 } from "@lucide/vue";
import type { BoardOrientation } from "@/lib/preferences";
import { usePreferencesStore } from "@/stores/preferences";

const preferences = usePreferencesStore();
const props = defineProps<{ openPosition: () => void }>();

function toggleCoordinates() {
  preferences.showCoordinates = !preferences.showCoordinates;
}

function flipBoard() {
  if (preferences.boardOrientation === "red") {
    preferences.setBoardOrientation("black");
  } else if (preferences.boardOrientation === "black") {
    preferences.setBoardOrientation("red");
  } else {
    preferences.setBoardOrientation("red");
  }
}
</script>

<template>
  <div class="flex flex-wrap items-start justify-between gap-x-2 gap-y-1 border-t bg-card/60 px-3 py-1.5 text-xs select-none sm:items-center">
    <div class="flex min-w-0 flex-wrap items-center gap-2">
      <!-- 视角选择 -->
      <div class="flex items-center gap-1.5">
        <span class="text-body-sm text-muted-foreground">视角:</span>
        <div class="w-24 shrink-0">
          <Select
            :model-value="preferences.boardOrientation"
            @update:model-value="preferences.setBoardOrientation($event as BoardOrientation)"
          >
            <SelectTrigger class="h-6 px-2 text-body-sm" aria-label="棋盘视角">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="red">红方视角</SelectItem>
                <SelectItem value="black">黑方视角</SelectItem>
                <SelectItem value="follow_turn">跟随行棋</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>
      </div>

      <!-- 快捷翻转 -->
      <Button variant="ghost" size="sm" class="h-6 gap-1 px-2 text-body-sm" title="快速翻转棋盘视角 (F)" @click="flipBoard">
        <ArrowLeftRight class="size-3" />
        <span>翻转</span>
      </Button>

      <!-- 坐标开关 -->
      <Button
        variant="ghost"
        size="sm"
        class="h-6 px-2 text-body-sm"
        :class="preferences.showCoordinates ? 'text-primary font-medium' : 'text-muted-foreground'"
        title="切换坐标显示 (C)"
        @click="toggleCoordinates"
      >
        坐标 {{ preferences.showCoordinates ? "开" : "关" }}
      </Button>
    </div>

    <div class="ml-auto shrink-0">
      <Button variant="outline" size="sm" class="h-6 gap-1 px-2.5 text-body-sm" @click="props.openPosition()">
        <Edit3 class="size-3" />
        <span>自定义摆局</span>
      </Button>
    </div>
  </div>
</template>
