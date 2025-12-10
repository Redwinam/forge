<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue';
import { useEditor, EditorContent } from '@tiptap/vue-3';
import StarterKit from '@tiptap/starter-kit';
import { Markdown } from 'tiptap-markdown';
import Image from '@tiptap/extension-image';
import CustomImage from './editor/image-extension';
import Link from '@tiptap/extension-link';
import Placeholder from '@tiptap/extension-placeholder';
import Underline from '@tiptap/extension-underline';
import TextAlign from '@tiptap/extension-text-align';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import { Table } from '@tiptap/extension-table';
import { TableCell } from '@tiptap/extension-table-cell';
import { TableHeader } from '@tiptap/extension-table-header';
import { TableRow } from '@tiptap/extension-table-row';
import { invoke } from '@tauri-apps/api/core';
import yaml from 'js-yaml';

import EditorToolbar from './editor/EditorToolbar.vue';
import MetadataPanel from './editor/MetadataPanel.vue';

const props = defineProps<{
  content: string;
  filePath: string | null;
  isSaving?: boolean;
}>();

const emit = defineEmits<{
  (e: 'save', content: string): void;
}>();

const isUploading = ref(false);
const metadata = ref<Record<string, any>>({});
const showMetadata = ref(true);
const isRawMode = ref(false);
const rawContent = ref('');
const isDirty = ref(false);
const lastSavedContent = ref('');

const saveStatus = computed(() => {
  if (props.isSaving) return 'saving';
  if (isDirty.value) return 'unsaved';
  return 'saved';
});

// Parse frontmatter
const parseContent = (fullContent: string) => {
  const frontmatterRegex = /^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/;
  const match = fullContent.match(frontmatterRegex);

  if (match) {
    try {
      const yamlContent = match[1];
      let bodyContent = match[2] || '';
      const parsedData = yaml.load(yamlContent || '') as Record<string, any>;
      
      metadata.value = parsedData || {};
      
      return bodyContent;
    } catch (e) {
      console.error("Failed to parse YAML", e);
      metadata.value = {};
      return fullContent;
    }
  }

  metadata.value = {};
  return fullContent;
};

const stringifyContent = (body: string) => {
  if (Object.keys(metadata.value).length === 0) return body;
  try {
    const yamlString = yaml.dump(metadata.value);
    return `---\n${yamlString}---\n${body}`;
  } catch (e) {
    console.error("Failed to stringify YAML", e);
    return body;
  }
};

const uploadFile = async (file: File): Promise<string> => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = async (e) => {
      if (e.target?.result) {
        try {
          const arrayBuffer = e.target.result as ArrayBuffer;
          const bytes = Array.from(new Uint8Array(arrayBuffer));
          const ext = file.name.split('.').pop() || 'dat';
          
          const url = await invoke<string>('upload_file', {
            fileData: bytes,
            extension: ext
          });
          resolve(url);
        } catch (err) {
          reject(err);
        }
      } else {
        reject("Failed to read file");
      }
    };
    reader.onerror = (e) => reject(e);
    reader.readAsArrayBuffer(file);
  });
};

const uploadImage = async (file: File): Promise<string> => {
  // Using new Rust command instead of JS SDK
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = async (e) => {
      if (e.target?.result) {
        try {
          // Convert ArrayBuffer to number array for Rust
          const arrayBuffer = e.target.result as ArrayBuffer;
          const bytes = Array.from(new Uint8Array(arrayBuffer));
          
          // Get extension
          const ext = file.name.split('.').pop() || 'png';
          
          // Call Rust command
          const url = await invoke<string>('upload_image', {
            fileData: bytes,
            extension: ext
          });
          resolve(url);
        } catch (err) {
          reject(err);
        }
      } else {
        reject("Failed to read file");
      }
    };
    reader.onerror = (e) => reject(e);
    reader.readAsArrayBuffer(file);
  });
};

const editor = useEditor({
  content: parseContent(props.content),
  onUpdate: ({ editor }) => {
    const markdownBody = (editor.storage as any).markdown.getMarkdown();
    const fullContent = stringifyContent(markdownBody);
    isDirty.value = fullContent !== lastSavedContent.value;
  },
  extensions: [
    StarterKit.configure({
      heading: {
        levels: [1, 2, 3, 4, 5, 6],
      },
    }),
    Markdown.configure({
      html: false, // Prevent HTML parsing which might confuse the parser
      tightLists: true, // Render tight lists
      transformPastedText: true, // Allow pasting markdown
      transformCopiedText: true, // Allow copying markdown
      linkify: true, // Auto-link URLs
      breaks: true, // Convert newlines to hard breaks
    }),
    CustomImage,
    Link.configure({
      openOnClick: false,
    }),
    Placeholder.configure({
      placeholder: "Type '/' for commands",
    }),
    Underline,
    TextAlign.configure({
      types: ['heading', 'paragraph'],
    }),
    TaskList.configure({
      HTMLAttributes: {
        class: 'not-prose pl-2',
      },
    }),
    TaskItem.configure({
      nested: true,
      HTMLAttributes: {
        class: 'task-list-item', // Use a dedicated class for styling
      },
    }),
    Table.configure({
      resizable: true,
    }),
    TableRow,
    TableHeader,
    TableCell,
  ],
  editorProps: {
    attributes: {
      class: 'prose prose-slate max-w-none focus:outline-none min-h-[calc(100vh-200px)] px-8 py-6'
    },
    handlePaste: (_view, event) => {
      const items = Array.from(event.clipboardData?.items || []);
      const item = items.find(item => item.type.indexOf('image') === 0);
      if (item) {
        const file = item.getAsFile();
        if (file) {
          event.preventDefault();
          isUploading.value = true;
          uploadImage(file)
            .then(url => {
              editor.value?.chain().focus().setImage({ src: url }).run();
            })
            .catch(err => alert("Upload failed: " + err))
            .finally(() => isUploading.value = false);
          return true;
        }
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
watch(() => props.content, (newContent) => {
  lastSavedContent.value = newContent;
  isDirty.value = false;
  rawContent.value = newContent;

  const newBody = parseContent(newContent);
  if (editor.value && newBody !== (editor.value.storage as any).markdown.getMarkdown()) {
    editor.value.commands.setContent(newBody || '');
  }
}, { immediate: true });

const setLink = () => {
  if (!editor.value) return;
  const previousUrl = editor.value.getAttributes('link').href;
  const url = window.prompt('URL', previousUrl);

  // cancelled
  if (url === null) {
    return;
  }

  // empty
  if (url === '') {
    editor.value.chain().focus().extendMarkRange('link').unsetLink().run();
    return;
  }

  // update
  editor.value.chain().focus().extendMarkRange('link').setLink({ href: url }).run();
};

const insertTable = () => {
  editor.value?.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run();
};

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

const handleFileUploadClick = () => {
  const input = document.createElement('input');
  input.type = 'file';
  input.onchange = async (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (file) {
      isUploading.value = true;
      try {
        const url = await uploadFile(file);
        if (editor.value && !isRawMode.value) {
            editor.value.chain().focus().insertContent(`[${file.name}](${url})`).run();
        } else if (isRawMode.value) {
            rawContent.value += `\n[${file.name}](${url})`;
        }
      } catch (err) {
        alert("Upload failed: " + err);
      } finally {
        isUploading.value = false;
      }
    }
  };
  input.click();
};

const handleSave = () => {
  if (isRawMode.value) {
    const fullContent = rawContent.value;
    emit('save', fullContent);
    lastSavedContent.value = fullContent;
    isDirty.value = false;
  } else if (editor.value) {
    const markdownBody = (editor.value.storage as any).markdown.getMarkdown();
    const fullContent = stringifyContent(markdownBody);
    emit('save', fullContent);
    lastSavedContent.value = fullContent;
    isDirty.value = false;
  }
};

const toggleMode = () => {
  if (isRawMode.value) {
    const newBody = parseContent(rawContent.value);
    editor.value?.commands.setContent(newBody || '');
    isRawMode.value = false;
  } else {
    if (editor.value) {
      const markdownBody = (editor.value.storage as any).markdown.getMarkdown();
      rawContent.value = stringifyContent(markdownBody);
    }
    isRawMode.value = true;
  }
};

const handleKeyDown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 's') {
    e.preventDefault();
    handleSave();
  }
};

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeyDown);
  editor.value?.destroy();
});

const getFullContent = () => {
  if (isRawMode.value) {
    return rawContent.value;
  }
  if (editor.value) {
    const markdownBody = (editor.value.storage as any).markdown.getMarkdown();
    return stringifyContent(markdownBody);
  }
  return rawContent.value;
};

const setSaved = (content: string) => {
  lastSavedContent.value = content;
  isDirty.value = false;
};

defineExpose({
  save: handleSave,
  getFullContent,
  setSaved,
  isDirty,
});
</script>

<template>
  <div class="flex flex-col h-full relative bg-white">
    <EditorToolbar 
      :editor="editor" 
      :show-metadata="showMetadata"
      :save-status="saveStatus"
      :is-raw-mode="isRawMode"
      @toggle-metadata="showMetadata = !showMetadata"
      @upload-image="handleImageUploadClick"
      @set-link="setLink"
      @insert-table="insertTable"
      @save="handleSave"
      @toggle-mode="toggleMode"
      @upload-file="handleFileUploadClick"
    />

    <MetadataPanel 
      :metadata="metadata" 
      :show="showMetadata"
    />

    <div v-if="isUploading" class="absolute top-14 right-4 z-20 bg-blue-100 text-blue-700 px-3 py-1 rounded-full text-sm shadow-md animate-pulse">
      Uploading...
    </div>

    <div v-if="isRawMode" class="flex-1 overflow-y-auto bg-gray-50 p-4">
      <textarea 
        v-model="rawContent" 
        class="w-full h-full p-4 font-mono text-sm bg-white border border-gray-200 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        spellcheck="false"
        @input="isDirty = true"
      ></textarea>
    </div>

    <editor-content v-else :editor="editor" class="flex-1 overflow-y-auto" />
  </div>
</template>

<style>
/* Custom Scrollbar */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: #94a3b8;
}

/* Editor Specifics */
.ProseMirror {
  outline: none;
  min-height: 100%;
}

.ProseMirror p.is-editor-empty:first-child::before {
  color: #94a3b8;
  content: attr(data-placeholder);
  float: left;
  height: 0;
  pointer-events: none;
}

/* Typography Overrides */
.ProseMirror h1,
.ProseMirror h2,
.ProseMirror h3,
.ProseMirror h4,
.ProseMirror h5,
.ProseMirror h6 {
  line-height: 1.1;
  margin-top: 2.5rem;
  margin-bottom: 0.5rem;
  text-wrap: pretty;
}

.ProseMirror h1 {
  font-size: 2.25em;
  font-weight: 800;
  color: #111827;
}
.ProseMirror h2 {
  font-size: 1.5em;
  font-weight: 700;
  color: #1f2937;
}
.ProseMirror h3 {
  font-size: 1.25em;
  font-weight: 600;
  color: #374151;
}

.ProseMirror p {
  margin-top: 0.75em;
  margin-bottom: 0.75em;
  line-height: 1.75;
}

/* Remove backticks from inline code in Tailwind Typography */
.prose :where(code):not(:where([class~="not-prose"] *))::before {
  content: "" !important;
}
.prose :where(code):not(:where([class~="not-prose"] *))::after {
  content: "" !important;
}

.ProseMirror code {
  background-color: #f1f5f9;
  color: #0f172a;
  border-radius: 0.25rem;
  padding: 0.25rem 0.375rem;
  font-size: 0.875em;
  font-weight: 500;
}

.ProseMirror pre {
  background: #1f2937;
  color: #e5e7eb;
  font-family: 'JetBrainsMono', monospace;
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
  margin: 1.5rem 0;
}
.ProseMirror pre code {
  color: inherit;
  padding: 0;
  background: none;
  font-size: 0.8rem;
  font-weight: normal;
}

/* 
   TASK LIST STYLING 
   This aims to be a robust, pixel-perfect alignment.
*/

ul[data-type="taskList"] {
  list-style: none;
  padding: 0;
  margin: 0;
}

/* The item container */
li.task-list-item {
  display: flex;
  flex-direction: row;
  align-items: flex-start; /* Aligns checkbox with the top of the content */
  margin-bottom: 0.2rem;
}

/* The checkbox label container */
li.task-list-item > label {
  flex: 0 0 auto; /* Don't shrink or grow */
  margin-right: 0.5rem;
  user-select: none;
  /* Fixed height equal to line-height to center checkbox vertically */
  height: 1.75rem; 
  display: flex;
  align-items: center;
  justify-content: center;
}

/* The actual checkbox input */
li.task-list-item input[type="checkbox"] {
  /* Reset appearance */
  appearance: none;
  -webkit-appearance: none;
  background-color: #fff;
  margin: 0;
  cursor: pointer;
  width: 1.1rem;
  height: 1.1rem;
  border: 1.5px solid #cbd5e1;
  border-radius: 0.3rem;
  display: grid;
  place-content: center;
  transition: all 0.1s ease-in-out;
}

li.task-list-item input[type="checkbox"]::before {
  content: "";
  width: 0.65rem;
  height: 0.65rem;
  transform: scale(0);
  transition: 120ms transform ease-in-out;
  box-shadow: inset 1em 1em white;
  transform-origin: center;
  clip-path: polygon(14% 44%, 0 65%, 50% 100%, 100% 16%, 80% 0%, 43% 62%);
}

li.task-list-item input[type="checkbox"]:checked {
  background-color: #3b82f6; /* Blue-500 */
  border-color: #3b82f6;
}

li.task-list-item input[type="checkbox"]:checked::before {
  transform: scale(1);
}

/* The content container */
li.task-list-item > div {
  flex: 1 1 auto;
  min-width: 0; /* Prevents overflow */
}

/* Ensure paragraphs inside don't have margins that break alignment */
li.task-list-item > div > p {
  margin-top: 0;
  margin-bottom: 0;
  line-height: 1.75rem;
}

/* Blockquote */
.ProseMirror blockquote {
  border-left: 3px solid #e2e8f0;
  padding-left: 1rem;
  font-style: italic;
  color: #4b5563;
  margin-left: 0;
  margin-right: 0;
}

/* Images */
/* handled by ImageNode.vue now, but keep generic styles just in case */
.ProseMirror img {
  max-width: 100%;
  height: auto;
  border-radius: 0.5rem;
}

/* Links */
.ProseMirror a {
  color: #2563eb;
  text-decoration: none;
  cursor: pointer;
}
.ProseMirror a:hover {
  text-decoration: underline;
}

.ProseMirror ul,
.ProseMirror ol {
  padding-left: 1.5rem;
}

/* Tables */
.ProseMirror table {
  border-collapse: collapse;
  table-layout: fixed;
  width: 100%;
  margin: 0;
  overflow: hidden;
}

.ProseMirror td,
.ProseMirror th {
  min-width: 1em;
  border: 1px solid #ced4da;
  padding: 0.5rem;
  vertical-align: top;
  box-sizing: border-box;
  position: relative;
}

.ProseMirror th {
  font-weight: 600;
  text-align: left;
  background-color: #f8f9fa;
}

.ProseMirror .selectedCell:after {
  z-index: 2;
  position: absolute;
  content: "";
  left: 0; right: 0; top: 0; bottom: 0;
  background: rgba(200, 200, 255, 0.4);
  pointer-events: none;
}

.ProseMirror .column-resize-handle {
  position: absolute;
  right: -2px;
  top: 0;
  bottom: -2px;
  width: 4px;
  background-color: #adf;
  pointer-events: none;
}
</style>
