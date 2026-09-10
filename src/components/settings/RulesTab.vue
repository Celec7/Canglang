<script setup lang="ts">
import type { RuleProfile } from "@/bindings";
import type { RuleProfileType } from "@/lib/preferences";
import { useGameStore } from "@/stores/game";
import { usePreferencesStore } from "@/stores/preferences";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui";
import SettingRow from "../SettingRow.vue";

const game = useGameStore();
const preferences = usePreferencesStore();
</script>

<template>
  <div class="flex flex-col gap-2">
    <h4 class="text-xs font-semibold text-foreground mb-1">官方竞赛裁判标准</h4>
    <SettingRow label="规则档案" description="决定长将、长捉、互见循环等往返着法的仲裁基准">
      <Select :model-value="game.ruleProfile ?? preferences.defaultRuleProfile" :disabled="game.variant !== 'xiangqi'" @update:model-value="game.setRuleProfile($event as RuleProfile); preferences.setDefaultRuleProfile($event as RuleProfileType)">
        <SelectTrigger class="h-8 w-full text-xs sm:w-56" aria-label="规则档案"><SelectValue /></SelectTrigger>
        <SelectContent><SelectGroup><SelectItem value="china2020">中国象棋竞赛规则 (2020版)</SelectItem><SelectItem value="asian2017">亚洲象棋联合会比赛规则 (2017版)</SelectItem></SelectGroup></SelectContent>
      </Select>
    </SettingRow>
    <div class="rounded-lg border p-3 bg-muted/20 text-xs text-muted-foreground leading-relaxed mt-2 flex flex-col gap-1.5">
      <div class="font-medium text-foreground">关于规则与判决：</div>
      <div>• <strong>绝杀与困毙：</strong>被将军无合法解法或轮到走子方无子可走，均判负。</div>
      <div>• <strong>长打裁决：</strong>单方面长将判负，长捉在不同规则档案下有细微例外与自毙判定，系统将在产生循环时给出明确裁判解释。</div>
    </div>
  </div>
</template>
