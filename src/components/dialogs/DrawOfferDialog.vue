<script setup lang="ts">
import { computed, ref } from "vue";
import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui";
import { useGameStore } from "@/stores/game";

const game = useGameStore();
const busy = ref(false);
const offer = computed(() => game.snapshot?.draw_offer ?? null);
const currentSide = computed(() => game.redToMove ? "red" : "black");
const isProposer = computed(() => offer.value?.proposer === currentSide.value);

async function act(action: "accept" | "reject" | "cancel") {
  if (!offer.value || busy.value) return;
  busy.value = true;
  try {
    if (action === "cancel") await game.cancelDraw(offer.value.id, currentSide.value);
    else await game.respondDraw(offer.value.id, currentSide.value, action === "accept");
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <Dialog :open="Boolean(offer)">
    <DialogContent class="max-w-sm">
      <DialogHeader>
        <DialogTitle>{{ isProposer ? "求和已提出" : "对方请求和棋" }}</DialogTitle>
        <DialogDescription>
          {{ isProposer ? "等待对方回应。回应前对局暂停，你可以取消请求。" : "接受后本局立即判和；拒绝后继续对局。" }}
        </DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <Button v-if="isProposer" variant="outline" :disabled="busy" @click="act('cancel')">取消求和</Button>
        <template v-else>
          <Button variant="outline" :disabled="busy" @click="act('reject')">拒绝</Button>
          <Button :disabled="busy" @click="act('accept')">同意和棋</Button>
        </template>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
