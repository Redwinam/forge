<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Settings, ChevronLeft, Menu } from 'lucide-vue-next';
import FileTreeItem from './FileTreeItem.vue';

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileEntry[];
}

const props = defineProps<{
  rootPath: string;
  currentFile: string | null;
}>();

const emit = defineEmits<{
  (e: 'select', path: string): void;
  (e: 'open-settings'): void;
}>();

const rootFiles = ref<FileEntry[]>([]);
const width = ref(260);
const isResizing = ref(false);
const isCollapsed = ref(false);
const sidebarRef = ref<HTMLDivElement | null>(null);

watch(() => props.rootPath, (newPath) => {
  if (newPath) {
    invoke<FileEntry[]>('list_files', { path: newPath })
      .then(files => rootFiles.value = files)
      .catch(console.error);
  }
}, { immediate: true });

const startResizing = () => {
  isResizing.value = true;
};

const stopResizing = () => {
  isResizing.value = false;
};

const resize = (e: MouseEvent) => {
  if (isResizing.value) {
    const newWidth = e.clientX;
    if (newWidth > 150 && newWidth < 600) {
      width.value = newWidth;
    }
  }
};

onMounted(() => {
  window.addEventListener('mousemove', resize);
  window.addEventListener('mouseup', stopResizing);
  console.log("Sidebar mounted", sidebarRef.value);
});

onUnmounted(() => {
  window.removeEventListener('mousemove', resize);
  window.removeEventListener('mouseup', stopResizing);
});

const handleSelect = (path: string) => {
  emit('select', path);
};
</script>

<template>
  <div v-if="isCollapsed" class="w-12 h-full bg-gray-50 border-r border-gray-200 flex flex-col items-center py-4">
    <button 
      @click="isCollapsed = false"
      class="p-2 hover:bg-gray-200 rounded-md text-gray-600 mb-4"
      title="Expand Sidebar"
    >
      <Menu :size="20" />
    </button>
  </div>

  <div 
    v-else
    ref="sidebarRef"
    class="h-full bg-gray-50 flex flex-col relative group select-none"
    :style="{ width: `${width}px` }"
  >
    <!-- Header -->
    <div class="h-10 flex items-center justify-between px-3 border-b border-gray-200 shrink-0">
      <span class="text-xs font-semibold text-gray-500 uppercase tracking-wider truncate" :title="rootPath">
        {{ rootPath ? rootPath.split('/').pop() : 'Files' }}
      </span>
      <div class="flex items-center">
        <button 
          @click="$emit('open-settings')"
          class="p-1 text-gray-400 hover:text-gray-700 hover:bg-gray-200 rounded-md transition-colors"
          title="Settings"
        >
          <Settings :size="14" />
        </button>
        <button 
          @click="isCollapsed = true"
          class="p-1 text-gray-400 hover:text-gray-700 hover:bg-gray-200 rounded-md transition-colors ml-1"
          title="Collapse"
        >
          <ChevronLeft :size="14" />
        </button>
      </div>
    </div>

    <!-- File Tree -->
    <div class="flex-1 overflow-y-auto py-2 custom-scrollbar">
      <FileTreeItem
        v-for="file in rootFiles"
        :key="file.path"
        :entry="file"
        :depth="0"
        :currentFile="currentFile"
        @select="handleSelect"
      />
    </div>

    <!-- Resize Handle -->
    <div
      class="absolute top-0 right-0 w-1 h-full cursor-col-resize hover:bg-blue-400 transition-colors z-20"
      @mousedown="startResizing"
    ></div>
    <!-- Visual border -->
    <div class="absolute top-0 right-0 w-px h-full bg-gray-200 pointer-events-none"></div>
  </div>
</template>
