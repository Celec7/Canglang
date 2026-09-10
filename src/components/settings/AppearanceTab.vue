<script setup lang="ts">
import type { BoardOrientation, ThemeMode } from "@/lib/preferences";
import { usePreferencesStore } from "@/stores/preferences";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue, Separator, Slider, Switch } from "@/components/ui";
import SettingRow from "../SettingRow.vue";

const preferences = usePreferencesStore();
</script>

<template>
  <div class="flex flex-col gap-2">
    <h4 class="text-xs font-semibold text-foreground mb-1">主题与视界</h4>
    <SettingRow label="色彩主题" description="浅色雅致或深色高对比工作区">
      <Select :model-value="preferences.theme" @update:model-value="preferences.setTheme($event as ThemeMode)">
        <SelectTrigger class="h-8 w-full text-xs sm:w-32" aria-label="色彩主题"><SelectValue /></SelectTrigger>
        <SelectContent><SelectGroup><SelectItem value="light">浅色模式</SelectItem><SelectItem value="dark">深色模式</SelectItem></SelectGroup></SelectContent>
      </Select>
    </SettingRow>
    <SettingRow label="默认视角" description="红方底线、黑方底线或随行棋方自动翻转">
      <Select :model-value="preferences.boardOrientation" @update:model-value="preferences.setBoardOrientation($event as BoardOrientation)">
        <SelectTrigger class="h-8 w-full text-xs sm:w-32" aria-label="默认视角"><SelectValue /></SelectTrigger>
        <SelectContent><SelectGroup><SelectItem value="red">红方视角</SelectItem><SelectItem value="black">黑方视角</SelectItem><SelectItem value="follow_turn">跟随走子</SelectItem></SelectGroup></SelectContent>
      </Select>
    </SettingRow>
    <SettingRow label="棋盘边缘路数坐标" description="在棋盘外侧标注一~九与 1~9 纵线路号">
      <Switch aria-label="棋盘边缘路数坐标" :checked="preferences.showCoordinates" @update:checked="preferences.showCoordinates = $event" />
    </SettingRow>
    <SettingRow label="引擎走法指示箭头" description="在棋盘上渲染引擎最佳候选着法箭头">
      <Switch aria-label="引擎走法指示箭头" :checked="preferences.showEngineArrow" @update:checked="preferences.showEngineArrow = $event" />
    </SettingRow>
    <Separator class="my-2" />
    <h4 class="text-xs font-semibold text-foreground mb-1">音效与动效</h4>
    <SettingRow label="落子敲击音效" description="移动、吃子与将军时的纯净木质敲击音效">
      <Switch aria-label="落子敲击音效" :checked="preferences.soundEnabled" @update:checked="preferences.soundEnabled = $event" />
    </SettingRow>
    <SettingRow label="走子过渡动效" description="开启后平滑滑动落位，关闭则瞬间落子">
      <Switch aria-label="走子过渡动效" :checked="preferences.animations" @update:checked="preferences.animations = $event" />
    </SettingRow>
    <SettingRow v-if="preferences.animations" label="动效滑动时长" :description="`当前：${preferences.moveAnimationSeconds.toFixed(2)} 秒（推荐 0.15~0.25 秒）`">
      <div class="flex w-full items-center gap-3 sm:w-44">
        <Slider aria-label="动效滑动时长" :model-value="[preferences.moveAnimationSeconds]" :min="0.05" :max="0.5" :step="0.05" class="flex-1" @update:model-value="preferences.setMoveAnimationSeconds($event?.[0] ?? 0.2)" />
        <span class="w-10 text-right text-xs font-mono tabular-nums text-muted-foreground">{{ preferences.moveAnimationSeconds.toFixed(2) }}s</span>
      </div>
    </SettingRow>
  </div>
</template>
