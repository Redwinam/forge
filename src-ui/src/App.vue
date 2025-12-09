<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { LazyStore } from '@tauri-apps/plugin-store';
import Sidebar from './components/Sidebar.vue';
import Editor from './components/Editor.vue';
import SettingsModal from './components/SettingsModal.vue';

const store = new LazyStore('.settings.dat');
const currentPath = ref<string | null>(null);
const currentContent = ref('');
const isSettingsOpen = ref(false);
const isSaving = ref(false);

const rootPath = ref(localStorage.getItem('forge_root_path') || '/Users/redwinam/Developer/notes/Press/docs');

const loadLastOpenedFile = async () => {
  try {
    const lastFile = await store.get<string>('last_opened_file');
    if (lastFile) {
      handleFileSelect(lastFile);
    }
  } catch (e) {
    console.error("Failed to load last opened file", e);
  }
};

onMounted(() => {
  loadLastOpenedFile();
});

watch(currentPath, async (newPath) => {
  if (newPath) {
    await store.set('last_opened_file', newPath);
    await store.save();
  }
});

const handleFileSelect = async (path: string) => {
  try {
    const content = await invoke<string>('read_file', { path });
    currentPath.value = path;
    currentContent.value = content;
  } catch (e) {
    console.error("Failed to read file", e);
    alert("Failed to read file: " + e);
  }
};

const handleSave = async (content: string) => {
  if (!currentPath.value) return;
  isSaving.value = true;
  try {
    await invoke('write_file', { path: currentPath.value, content });
    console.log("Saved!");
    // Update currentContent to reflect saved state if needed, though Editor handles it
  } catch (e) {
    console.error("Failed to save", e);
    alert("Failed to save: " + e);
  } finally {
    isSaving.value = false;
  }
};

const handleSaveSettings = (newRootPath: string) => {
  rootPath.value = newRootPath;
  localStorage.setItem('forge_root_path', newRootPath);
};
</script>

<template>
  <div class="flex h-screen w-screen bg-white text-black overflow-hidden">
    <Sidebar 
      :rootPath="rootPath" 
      :currentFile="currentPath"
      @select="handleFileSelect"
      @open-settings="isSettingsOpen = true"
    />
    <div class="flex-1 flex flex-col h-full overflow-hidden">
      <Editor 
        v-if="currentPath"
        :key="currentPath"
        :content="currentContent"
        :filePath="currentPath"
        :isSaving="isSaving"
        @save="handleSave"
        class="flex-1"
      />
      <div v-else class="flex items-center justify-center h-full text-gray-400">
        Select a file to edit
      </div>
    </div>
    
    <SettingsModal 
      :isOpen="isSettingsOpen"
      :rootPath="rootPath"
      @close="isSettingsOpen = false"
      @save="handleSaveSettings"
    />
  </div>
</template>
