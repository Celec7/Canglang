<script setup lang="ts">
import {
  Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
} from "@/components/ui";
import { useManualStore } from "@/stores/manual";

const manual = useManualStore();

function handleOpen(open: boolean) {
  if (!open && manual.discardPromptOpen && !manual.resolvingDiscard) {
    void manual.resolveDiscard("cancel");
  }
}
</script>

<template>
  <Dialog :open="manual.discardPromptOpen" @update:open="handleOpen">
    <DialogContent class="max-w-md">
      <DialogHeader>
        <DialogTitle>保存未完成的更改？</DialogTitle>
        <DialogDescription>
          当前棋局或文档有尚未保存的内容。保存会写入当前原生格式；取消将保留当前工作。
        </DialogDescription>
      </DialogHeader>
      <p v-if="manual.error" role="alert" class="text-xs text-destructive">{{ manual.error }}</p>
      <DialogFooter class="gap-2 sm:justify-between">
        <Button variant="ghost" :disabled="manual.resolvingDiscard" @click="manual.resolveDiscard('cancel')">取消</Button>
        <div class="flex gap-2">
          <Button variant="outline" :disabled="manual.resolvingDiscard" @click="manual.resolveDiscard('discard')">放弃更改</Button>
          <Button :disabled="manual.resolvingDiscard" @click="manual.resolveDiscard('save')">
            {{ manual.resolvingDiscard ? "正在保存…" : "保存并继续" }}
          </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
