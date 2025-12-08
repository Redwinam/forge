<script setup lang="ts">
import { ref, watch } from 'vue';
import { X } from 'lucide-vue-next';

const props = defineProps<{
  isOpen: boolean;
  rootPath: string;
  envPath: string;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', rootPath: string, envPath: string): void;
}>();

const localRootPath = ref(props.rootPath);
const localEnvPath = ref(props.envPath);

watch(() => props.isOpen, (isOpen) => {
  if (isOpen) {
    localRootPath.value = props.rootPath;
    localEnvPath.value = props.envPath;
  }
});

const handleSave = () => {
  emit('save', localRootPath.value, localEnvPath.value);
  emit('close');
};
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="bg-white rounded-lg shadow-xl w-[500px] max-w-full m-4 overflow-hidden">
      <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200">
        <h2 class="text-lg font-semibold text-gray-900">Settings</h2>
        <button @click="$emit('close')" class="text-gray-400 hover:text-gray-600">
          <X :size="20" />
        </button>
      </div>
      
      <div class="p-6 space-y-4">
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

        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">
            .env File Path (Tencent COS)
          </label>
          <input
            type="text"
            v-model="localEnvPath"
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm"
            placeholder="/path/to/.env"
          />
          <p class="mt-1 text-xs text-gray-500">
            Path to .env file containing TENCENT_SECRET_ID, TENCENT_SECRET_KEY, etc.
          </p>
        </div>
      </div>

      <div class="flex items-center justify-end gap-3 px-6 py-4 bg-gray-50 border-t border-gray-200">
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
