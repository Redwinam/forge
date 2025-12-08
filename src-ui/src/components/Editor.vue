<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue';
import { useEditor, EditorContent } from '@tiptap/vue-3';
import StarterKit from '@tiptap/starter-kit';
import { Markdown } from 'tiptap-markdown';
import Image from '@tiptap/extension-image';
import Link from '@tiptap/extension-link';
import Placeholder from '@tiptap/extension-placeholder';
import { invoke } from '@tauri-apps/api/core';
import COS from 'cos-js-sdk-v5';
import SparkMD5 from 'spark-md5';
import { Bold, Italic, Strikethrough, Code, List, ListOrdered, Quote, Undo, Redo, Image as ImageIcon } from 'lucide-vue-next';

interface CosConfig {
  secret_id: string;
  secret_key: string;
  bucket: string;
  region: string;
  prefix: string;
}

const props = defineProps<{
  content: string;
  filePath: string | null;
  envPath: string;
}>();

const emit = defineEmits<{
  (e: 'save', content: string): void;
}>();

const cosConfig = ref<CosConfig | null>(null);
const isUploading = ref(false);

onMounted(() => {
  if (props.envPath) {
    invoke<CosConfig>('get_cos_config', { envPath: props.envPath })
      .then(config => cosConfig.value = config)
      .catch(console.error);
  }
});

const uploadImage = async (file: File): Promise<string> => {
  if (!cosConfig.value) throw new Error("COS Config not loaded");

  const cos = new COS({
    SecretId: cosConfig.value.secret_id,
    SecretKey: cosConfig.value.secret_key,
  });

  const calculateMD5 = (file: File): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      const spark = new SparkMD5.ArrayBuffer();

      reader.onload = (e) => {
        if (e.target?.result) {
          spark.append(e.target.result as ArrayBuffer);
          resolve(spark.end());
        } else {
          reject("Failed to read file");
        }
      };

      reader.onerror = (e) => reject(e);
      reader.readAsArrayBuffer(file);
    });
  };

  const hash = await calculateMD5(file);
  const ext = file.name.split('.').pop() || 'png';
  const key = `${cosConfig.value.prefix}images/${hash}.${ext}`;

  return new Promise((resolve, reject) => {
    cos.headObject({
      Bucket: cosConfig.value!.bucket,
      Region: cosConfig.value!.region,
      Key: key
    }, (_err, data) => {
      if (data) {
        const url = `https://cdn.if9.cool/${key}`;
        console.log("File already exists, skipping upload:", url);
        resolve(url);
        return;
      }

      cos.putObject({
        Bucket: cosConfig.value!.bucket,
        Region: cosConfig.value!.region,
        Key: key,
        Body: file,
      }, (err, _data) => {
        if (err) {
          console.error("COS Upload Error:", err);
          return reject(err);
        }
        const url = `https://cdn.if9.cool/${key}`;
        resolve(url);
      });
    });
  });
};

const editor = useEditor({
  content: props.content,
  extensions: [
    StarterKit,
    Markdown,
    Image,
    Link.configure({
      openOnClick: false,
    }),
    Placeholder.configure({
      placeholder: "Type '/' for commands",
    }),
  ],
  editorProps: {
    handlePaste: (_view, event) => {
      const items = Array.from(event.clipboardData?.items || []);
      const item = items.find(item => item.type.indexOf('image') === 0);
      if (item) {
        event.preventDefault();
        const file = item.getAsFile();
        if (file) {
          isUploading.value = true;
          uploadImage(file)
            .then(url => {
              editor.value?.chain().focus().setImage({ src: url }).run();
            })
            .catch(err => alert("Upload failed: " + err))
            .finally(() => isUploading.value = false);
        }
        return true;
      }
      return false;
    },
    handleDrop: (_view, event, _slice, moved) => {
      if (!moved && event.dataTransfer && event.dataTransfer.files && event.dataTransfer.files.length > 0) {
        const files = Array.from(event.dataTransfer.files);
        const image = files.find(file => file.type.indexOf('image') === 0);
        if (image) {
          event.preventDefault();
          isUploading.value = true;
          uploadImage(image)
            .then(url => {
              editor.value?.chain().focus().setImage({ src: url }).run();
            })
            .catch(err => alert("Upload failed: " + err))
            .finally(() => isUploading.value = false);
          return true;
        }
      }
      return false;
    }
  }
});

// Update content when props change
import { watch } from 'vue';
watch(() => props.content, (newContent) => {
  if (editor.value && newContent !== (editor.value.storage as any).markdown.getMarkdown()) {
    editor.value.commands.setContent(newContent);
  }
});

const handleImageUploadClick = () => {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = 'image/*';
  input.onchange = async (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (file) {
      isUploading.value = true;
      try {
        const url = await uploadImage(file);
        editor.value?.chain().focus().setImage({ src: url }).run();
      } catch (err) {
        alert("Upload failed: " + err);
      } finally {
        isUploading.value = false;
      }
    }
  };
  input.click();
};

const handleKeyDown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 's') {
    e.preventDefault();
    if (editor.value) {
      const markdown = (editor.value.storage as any).markdown.getMarkdown();
      emit('save', markdown);
    }
  }
};

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeyDown);
  editor.value?.destroy();
});
</script>

<template>
  <div class="flex flex-col h-full relative">
    <div v-if="editor" class="flex items-center gap-1 p-2 border-b border-gray-200 bg-white sticky top-0 z-10 overflow-x-auto">
      <button @click="editor.chain().focus().toggleBold().run()" :class="{ 'bg-gray-200': editor.isActive('bold') }" class="p-1.5 rounded hover:bg-gray-100" title="Bold">
        <Bold :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleItalic().run()" :class="{ 'bg-gray-200': editor.isActive('italic') }" class="p-1.5 rounded hover:bg-gray-100" title="Italic">
        <Italic :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleStrike().run()" :class="{ 'bg-gray-200': editor.isActive('strike') }" class="p-1.5 rounded hover:bg-gray-100" title="Strikethrough">
        <Strikethrough :size="18" />
      </button>
      <div class="w-px h-6 bg-gray-200 mx-1"></div>
      <button @click="editor.chain().focus().toggleCode().run()" :class="{ 'bg-gray-200': editor.isActive('code') }" class="p-1.5 rounded hover:bg-gray-100" title="Code">
        <Code :size="18" />
      </button>
      <div class="w-px h-6 bg-gray-200 mx-1"></div>
      <button @click="editor.chain().focus().toggleBulletList().run()" :class="{ 'bg-gray-200': editor.isActive('bulletList') }" class="p-1.5 rounded hover:bg-gray-100" title="Bullet List">
        <List :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleOrderedList().run()" :class="{ 'bg-gray-200': editor.isActive('orderedList') }" class="p-1.5 rounded hover:bg-gray-100" title="Ordered List">
        <ListOrdered :size="18" />
      </button>
      <div class="w-px h-6 bg-gray-200 mx-1"></div>
      <button @click="editor.chain().focus().toggleBlockquote().run()" :class="{ 'bg-gray-200': editor.isActive('blockquote') }" class="p-1.5 rounded hover:bg-gray-100" title="Quote">
        <Quote :size="18" />
      </button>
      <div class="w-px h-6 bg-gray-200 mx-1"></div>
      <button @click="handleImageUploadClick" class="p-1.5 rounded hover:bg-gray-100" title="Upload Image">
        <ImageIcon :size="18" />
      </button>
      <div class="flex-1"></div>
      <button @click="editor.chain().focus().undo().run()" :disabled="!editor.can().undo()" class="p-1.5 rounded hover:bg-gray-100 disabled:opacity-50" title="Undo">
        <Undo :size="18" />
      </button>
      <button @click="editor.chain().focus().redo().run()" :disabled="!editor.can().redo()" class="p-1.5 rounded hover:bg-gray-100 disabled:opacity-50" title="Redo">
        <Redo :size="18" />
      </button>
    </div>

    <div v-if="isUploading" class="absolute top-14 right-4 z-20 bg-blue-100 text-blue-700 px-3 py-1 rounded-full text-sm shadow-md animate-pulse">
      Uploading image...
    </div>

    <editor-content :editor="editor" class="flex-1 overflow-y-auto px-8 py-6 prose prose-lg prose-blue max-w-none outline-none" />
  </div>
</template>

<style>
/* Tiptap specific styles */
.ProseMirror {
  outline: none;
  min-height: 100%;
}

.ProseMirror p.is-editor-empty:first-child::before {
  color: #adb5bd;
  content: attr(data-placeholder);
  float: left;
  height: 0;
  pointer-events: none;
}

/* Image styles */
.ProseMirror img {
  max-width: 100%;
  height: auto;
  border-radius: 0.5rem;
  margin: 1rem 0;
}

.ProseMirror blockquote {
  border-left: 3px solid #e5e7eb;
  padding-left: 1rem;
  font-style: italic;
  color: #4b5563;
}

.ProseMirror pre {
  background: #0d0d0d;
  color: #fff;
  font-family: 'JetBrainsMono', monospace;
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
}

.ProseMirror code {
  color: #616161;
  background-color: #f3f4f6;
  padding: 0.2rem 0.4rem;
  border-radius: 0.25rem;
  font-size: 0.875em;
}

.ProseMirror pre code {
  color: inherit;
  padding: 0;
  background: none;
  font-size: 0.8rem;
}
</style>
