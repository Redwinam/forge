import React, { useEffect, useState, useCallback } from 'react';
import { useEditor, EditorContent } from '@tiptap/react';
import StarterKit from '@tiptap/starter-kit';
import { Markdown } from 'tiptap-markdown';
import Image from '@tiptap/extension-image';
import Link from '@tiptap/extension-link';
import Placeholder from '@tiptap/extension-placeholder';
import { invoke } from '@tauri-apps/api/tauri';
import COS from 'cos-js-sdk-v5';
import SparkMD5 from 'spark-md5';
import { Toolbar } from './Toolbar';

interface EditorProps {
  content: string;
  filePath: string | null;
  onSave: (content: string) => void;
  className?: string;
  envPath: string;
}

interface CosConfig {
  secret_id: string;
  secret_key: string;
  bucket: string;
  region: string;
  prefix: string;
}

export const Editor: React.FC<EditorProps> = ({ content, filePath, onSave, className, envPath }) => {
  const [cosConfig, setCosConfig] = useState<CosConfig | null>(null);
  const [isUploading, setIsUploading] = useState(false);

  useEffect(() => {
    if (envPath) {
      invoke<CosConfig>('get_cos_config', { envPath })
        .then(setCosConfig)
        .catch(console.error);
    }
  }, [envPath]);

  const uploadImage = useCallback(async (file: File): Promise<string> => {
    if (!cosConfig) throw new Error("COS Config not loaded");

    const cos = new COS({
      SecretId: cosConfig.secret_id,
      SecretKey: cosConfig.secret_key,
    });

    // Calculate MD5 of the file
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
    const key = `${cosConfig.prefix}images/${hash}.${ext}`;

    return new Promise((resolve, reject) => {
      // Check if exists first to save bandwidth (optional but recommended for MD5)
      cos.headObject({
        Bucket: cosConfig.bucket,
        Region: cosConfig.region,
        Key: key
      }, (err, data) => {
        if (data) {
           const url = `https://cdn.if9.cool/${key}`;
           console.log("File already exists, skipping upload:", url);
           resolve(url);
           return;
        }

        // Upload if not exists
        cos.putObject({
          Bucket: cosConfig.bucket,
          Region: cosConfig.region,
          Key: key,
          Body: file,
        }, (err, data) => {
          if (err) {
            console.error("COS Upload Error:", err);
            return reject(err);
          }
          const url = `https://cdn.if9.cool/${key}`;
          resolve(url);
        });
      });
    });
  }, [cosConfig]);

  const editor = useEditor({
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
    content: content,
    onUpdate: ({ editor }) => {
      // Auto-save or just track changes? 
      // We rely on Ctrl+S for saving to disk, but here we can update local state if needed.
    },
    editorProps: {
      handlePaste: (view, event) => {
        const items = Array.from(event.clipboardData?.items || []);
        const item = items.find(item => item.type.indexOf('image') === 0);
        if (item) {
          event.preventDefault();
          const file = item.getAsFile();
          if (file) {
            setIsUploading(true);
            uploadImage(file)
              .then(url => {
                editor?.chain().focus().setImage({ src: url }).run();
              })
              .catch(err => alert("Upload failed: " + err))
              .finally(() => setIsUploading(false));
          }
          return true;
        }
        return false;
      },
      handleDrop: (view, event, slice, moved) => {
        if (!moved && event.dataTransfer && event.dataTransfer.files && event.dataTransfer.files.length > 0) {
          const files = Array.from(event.dataTransfer.files);
          const image = files.find(file => file.type.indexOf('image') === 0);
          if (image) {
            event.preventDefault();
            setIsUploading(true);
            uploadImage(image)
              .then(url => {
                editor?.chain().focus().setImage({ src: url }).run();
              })
              .catch(err => alert("Upload failed: " + err))
              .finally(() => setIsUploading(false));
            return true;
          }
        }
        return false;
      }
    }
  }, [cosConfig]); // Re-create editor if config changes? Actually dependencies are for hooks.

  // Update content when file changes
  useEffect(() => {
    if (editor && content !== editor.storage.markdown.getMarkdown()) {
      editor.commands.setContent(content);
    }
  }, [content, editor]);

  // Handle Save shortcut
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        if (editor) {
          const markdown = editor.storage.markdown.getMarkdown();
          onSave(markdown);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [editor, onSave]);

  if (!editor) {
    return null;
  }

  const handleImageUploadClick = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) {
        setIsUploading(true);
        try {
          const url = await uploadImage(file);
          editor.chain().focus().setImage({ src: url }).run();
        } catch (err) {
          alert("Upload failed: " + err);
        } finally {
          setIsUploading(false);
        }
      }
    };
    input.click();
  };

  return (
    <div className={`flex flex-col h-full ${className}`}>
      <Toolbar editor={editor} onUploadImage={handleImageUploadClick} />
      {isUploading && (
        <div className="absolute top-12 right-4 z-20 bg-blue-100 text-blue-700 px-3 py-1 rounded-full text-sm shadow-md animate-pulse">
          Uploading image...
        </div>
      )}

      <EditorContent editor={editor} className="flex-1 overflow-y-auto px-8 py-6 prose prose-lg prose-blue max-w-none outline-none" />
    </div>
  );
};
