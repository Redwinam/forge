<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { File, Folder, ChevronRight, ChevronDown } from 'lucide-vue-next';
import { clsx } from 'clsx';

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileEntry[];
}

const props = defineProps<{
  entry: FileEntry;
  depth: number;
  currentFile: string | null;
}>();

const emit = defineEmits<{
  (e: 'select', path: string): void;
}>();

const isOpen = ref(false);
const children = ref<FileEntry[]>([]);

const toggleOpen = async (e: MouseEvent) => {
  e.stopPropagation();
  if (props.entry.is_dir) {
    if (!isOpen.value && children.value.length === 0) {
      try {
        const files = await invoke<FileEntry[]>('list_files', { path: props.entry.path });
        children.value = files;
      } catch (e) {
        console.error(e);
      }
    }
    isOpen.value = !isOpen.value;
  } else {
    emit('select', props.entry.path);
  }
};

const handleSelect = (path: string) => {
  emit('select', path);
};
</script>

<template>
  <div>
    <div
      :class="clsx(
        'flex items-center py-1 px-2 cursor-pointer hover:bg-gray-100 text-sm select-none truncate rounded-md mx-1 my-0.5',
        currentFile === entry.path ? 'bg-blue-100 text-blue-700 font-medium' : 'text-gray-700'
      )"
      :style="{ paddingLeft: `${depth * 12 + 8}px` }"
      @click="toggleOpen"
    >
      <span 
        class="mr-1 text-gray-400 hover:text-gray-600 p-0.5 rounded-sm"
        @click.stop="entry.is_dir ? toggleOpen($event) : undefined"
      >
        <template v-if="entry.is_dir">
          <ChevronDown v-if="isOpen" :size="14" />
          <ChevronRight v-else :size="14" />
        </template>
        <span v-else class="w-3.5 inline-block"></span>
      </span>
      <span class="mr-2 text-gray-500">
        <Folder v-if="entry.is_dir" :size="16" :fill="isOpen ? '#9CA3AF' : 'none'" />
        <File v-else :size="16" />
      </span>
      <span class="truncate">{{ entry.name }}</span>
    </div>
    <div v-if="isOpen">
      <FileTreeItem
        v-for="child in children"
        :key="child.path"
        :entry="child"
        :depth="depth + 1"
        :currentFile="currentFile"
        @select="handleSelect"
      />
    </div>
  </div>
</template>
