<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import Sidebar from './components/Sidebar.vue';
import Editor from './components/Editor.vue';
import SettingsModal from './components/SettingsModal.vue';

const currentPath = ref<string | null>(null);
const currentContent = ref('');
const isSettingsOpen = ref(false);

const rootPath = ref(localStorage.getItem('forge_root_path') || '/Users/redwinam/Developer/notes/Press/docs');
const envPath = ref(localStorage.getItem('forge_env_path') || '/Users/redwinam/Developer/notes/Press/.env');

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
  try {
    await invoke('write_file', { path: currentPath.value, content });
    console.log("Saved!");
  } catch (e) {
    console.error("Failed to save", e);
    alert("Failed to save: " + e);
  }
};

const handleSaveSettings = (newRootPath: string, newEnvPath: string) => {
  rootPath.value = newRootPath;
  envPath.value = newEnvPath;
  localStorage.setItem('forge_root_path', newRootPath);
  localStorage.setItem('forge_env_path', newEnvPath);
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
        :envPath="envPath"
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
      :envPath="envPath"
      @close="isSettingsOpen = false"
      @save="handleSaveSettings"
    />
  </div>
</template>
