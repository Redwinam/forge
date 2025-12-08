<script setup lang="ts">
import { ref, watch, onMounted } from 'vue';
import { X } from 'lucide-vue-next';
import { LazyStore } from '@tauri-apps/plugin-store';

const props = defineProps<{
  isOpen: boolean;
  rootPath: string;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', rootPath: string): void;
}>();

const store = new LazyStore('.settings.dat');

const localRootPath = ref(props.rootPath);
const secretId = ref('');
const secretKey = ref('');
const bucket = ref('');
const region = ref('');
const cdnDomain = ref('');
const prefix = ref('press/');

watch(() => props.isOpen, async (isOpen) => {
  if (isOpen) {
    localRootPath.value = props.rootPath;
    await loadCosSettings();
  }
});

const loadCosSettings = async () => {
  try {
    secretId.value = await store.get<string>('cos_secret_id') || '';
    secretKey.value = await store.get<string>('cos_secret_key') || '';
    bucket.value = await store.get<string>('cos_bucket') || '';
    region.value = await store.get<string>('cos_region') || '';
    cdnDomain.value = await store.get<string>('cos_cdn_domain') || '';
    prefix.value = await store.get<string>('cos_prefix') || 'press/';
  } catch (e) {
    console.error("Failed to load COS settings", e);
  }
};

const handleSave = async () => {
  try {
    await store.set('cos_secret_id', secretId.value);
    await store.set('cos_secret_key', secretKey.value);
    await store.set('cos_bucket', bucket.value);
    await store.set('cos_region', region.value);
    await store.set('cos_cdn_domain', cdnDomain.value);
    await store.set('cos_prefix', prefix.value);
    await store.save();
    
    emit('save', localRootPath.value);
    emit('close');
  } catch (e) {
    console.error("Failed to save settings", e);
    alert("Failed to save settings: " + e);
  }
};

onMounted(() => {
  loadCosSettings();
});
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="bg-white rounded-lg shadow-xl w-[600px] max-w-full m-4 overflow-hidden flex flex-col max-h-[90vh]">
      <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 shrink-0">
        <h2 class="text-lg font-semibold text-gray-900">Settings</h2>
        <button @click="$emit('close')" class="text-gray-400 hover:text-gray-600">
          <X :size="20" />
        </button>
      </div>
      
      <div class="p-6 space-y-6 overflow-y-auto custom-scrollbar">
        <!-- General -->
        <div>
          <h3 class="text-sm font-bold text-gray-900 uppercase tracking-wider mb-3 pb-1 border-b border-gray-100">General</h3>
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">
              Documents Root Path
            </label>
            <input
              type="text"
              v-model="localRootPath"
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm"
              placeholder="/path/to/docs"
            />
            <p class="mt-1 text-xs text-gray-500">
              Absolute path to your markdown files directory.
            </p>
          </div>
        </div>

        <!-- Tencent COS -->
        <div>
          <h3 class="text-sm font-bold text-gray-900 uppercase tracking-wider mb-3 pb-1 border-b border-gray-100">Tencent COS</h3>
          <div class="grid grid-cols-1 gap-4">
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Secret ID</label>
              <input type="password" v-model="secretId" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm" />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Secret Key</label>
              <input type="password" v-model="secretKey" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm" />
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Bucket</label>
                <input type="text" v-model="bucket" placeholder="example-1234567890" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm" />
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Region</label>
                <input type="text" v-model="region" placeholder="ap-shanghai" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm" />
              </div>
            </div>
            <div class="grid grid-cols-2 gap-4">
               <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">CDN Domain (Optional)</label>
                <input type="text" v-model="cdnDomain" placeholder="Leave empty to use default COS domain" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm" />
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Prefix</label>
                <input type="text" v-model="prefix" placeholder="press/" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm" />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="flex items-center justify-end gap-3 px-6 py-4 bg-gray-50 border-t border-gray-200 shrink-0">
        <button
          @click="$emit('close')"
          class="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
        >
          Cancel
        </button>
        <button
          @click="handleSave"
          class="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
        >
          Save Changes
        </button>
      </div>
    </div>
  </div>
</template>
