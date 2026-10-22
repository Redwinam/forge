<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import Sidebar from './components/Sidebar.vue';
import Editor from './components/Editor.vue';
import SettingsModal from './components/SettingsModal.vue';
import { isInside } from './lib/paths';

interface AppState {
  rootDir: string | null;
  lastOpenedFile: string | null;
  sidebarWidth: number | null;
}

const currentPath = ref<string | null>(null);
const currentContent = ref('');
const isSettingsOpen = ref(false);
const isSaving = ref(false);
const editorRef = ref<any>(null);

const rootPath = ref<string | null>(null);
const sidebarWidth = ref<number | null>(null);

// Earlier versions kept the root in localStorage; the backend adopts it once and owns it from then on
const LEGACY_ROOT_KEY = 'forge_root_path';

onMounted(async () => {
  try {
    // Without a configured folder the backend falls back to the Documents folder
    const legacyRootDir = localStorage.getItem(LEGACY_ROOT_KEY);
    const state = await invoke<AppState>('load_app_state', { legacyRootDir });
    if (state.rootDir) localStorage.removeItem(LEGACY_ROOT_KEY);
    rootPath.value = state.rootDir;
    sidebarWidth.value = state.sidebarWidth;
    if (state.lastOpenedFile) {
      await reopenLastFile(state.lastOpenedFile);
    }
  } catch (e) {
    console.error("Failed to load app state", e);
  }
});

// The file may have been moved or deleted since; that's no reason to greet the user with an error
const reopenLastFile = async (path: string) => {
  try {
    currentContent.value = await invoke<string>('read_file', { path });
    currentPath.value = path;
  } catch (e) {
    console.warn("Last opened file is gone", e);
    await invoke('set_last_opened_file', { path: null }).catch(() => {});
  }
};

watch(currentPath, async (newPath) => {
  if (newPath) {
    try {
      await invoke('set_last_opened_file', { path: newPath });
    } catch (e) {
      console.error("Failed to remember last opened file", e);
    }
  }
});

// Saves the open note if it has unsaved edits; false means it couldn't be saved
const saveIfDirty = async (): Promise<boolean> => {
  if (!currentPath.value || !editorRef.value) return true;
  const isDirty = editorRef.value.isDirty;
  // Handle both ref (needs .value) and unwrapped boolean cases
  const needsSave = typeof isDirty === 'boolean' ? isDirty : isDirty?.value;
  if (!needsSave) return true;
  return handleSave(editorRef.value.getFullContent());
};

const handleFileSelect = async (path: string) => {
  try {
    // Stay on the current file if saving failed, otherwise the unsaved edits are lost
    if (!(await saveIfDirty())) return;

    const content = await invoke<string>('read_file', { path });
    currentPath.value = path;
    currentContent.value = content;
  } catch (e) {
    console.error("Failed to read file", e);
    alert("Failed to read file: " + e);
  }
};

const handleSave = async (content: string): Promise<boolean> => {
  const path = currentPath.value;
  if (!path) return false;
  isSaving.value = true;
  try {
    await invoke('write_file', { path, content });
    console.log("Saved!");
    // Only mark the editor as saved once the write succeeded
    if (currentPath.value === path) {
      editorRef.value?.setSaved(content);
    }
    return true;
  } catch (e) {
    console.error("Failed to save", e);
    alert("Failed to save: " + e);
    return false;
  } finally {
    isSaving.value = false;
  }
};

const handleChooseRoot = async () => {
  // The open note must be saved while it is still inside the current root
  if (!(await saveIfDirty())) return;
  try {
    const dir = await invoke<string | null>('choose_root_dir');
    if (!dir || dir === rootPath.value) return;
    rootPath.value = dir;
    if (currentPath.value && !isInside(currentPath.value, dir)) {
      currentPath.value = null;
      currentContent.value = '';
    }
  } catch (e) {
    console.error("Failed to choose notes folder", e);
    alert("Failed to choose notes folder: " + e);
  }
};
</script>

<template>
  <div class="flex h-screen w-screen bg-white text-black overflow-hidden">
    <Sidebar 
      :rootPath="rootPath" 
      :currentFile="currentPath"
      :initialWidth="sidebarWidth"
      @select="handleFileSelect"
      @choose-root="handleChooseRoot"
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
    ref="editorRef"
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
      @choose-root="handleChooseRoot"
    />
  </div>
</template>
