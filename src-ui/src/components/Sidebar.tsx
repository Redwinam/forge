import React, { useState, useEffect, useRef } from 'react';
import { Folder, File, ChevronRight, ChevronDown, Settings, ChevronLeft, Menu } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';
import clsx from 'clsx';

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileEntry[];
}

interface SidebarProps {
  rootPath: string;
  onFileSelect: (path: string) => void;
  currentFile: string | null;
  onOpenSettings: () => void;
}

const FileTreeItem: React.FC<{
  entry: FileEntry;
  depth: number;
  onSelect: (path: string) => void;
  currentFile: string | null;
}> = ({ entry, depth, onSelect, currentFile }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [children, setChildren] = useState<FileEntry[]>([]);

  const toggleOpen = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (entry.is_dir) {
      if (!isOpen && children.length === 0) {
        try {
          const files = await invoke<FileEntry[]>('list_files', { path: entry.path });
          setChildren(files);
        } catch (e) {
          console.error(e);
        }
      }
      setIsOpen(!isOpen);
    } else {
      onSelect(entry.path);
    }
  };

  return (
    <div>
      <div
        className={clsx(
          "flex items-center py-1 px-2 cursor-pointer hover:bg-gray-100 text-sm select-none truncate rounded-md mx-1 my-0.5",
          currentFile === entry.path ? "bg-blue-100 text-blue-700 font-medium" : "text-gray-700"
        )}
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
        onClick={(e) => {
           if (!entry.is_dir) onSelect(entry.path);
           else toggleOpen(e);
        }}
      >
        <span 
          className="mr-1 text-gray-400 hover:text-gray-600 p-0.5 rounded-sm"
          onClick={entry.is_dir ? toggleOpen : undefined}
        >
          {entry.is_dir ? (
            isOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />
          ) : (
            <span className="w-3.5 inline-block" />
          )}
        </span>
        <span className="mr-2 text-gray-500">
          {entry.is_dir ? <Folder size={16} fill={isOpen ? "#9CA3AF" : "none"} /> : <File size={16} />}
        </span>
        <span className="truncate">{entry.name}</span>
      </div>
      {isOpen && (
        <div>
          {children.map((child) => (
            <FileTreeItem
              key={child.path}
              entry={child}
              depth={depth + 1}
              onSelect={onSelect}
              currentFile={currentFile}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export const Sidebar: React.FC<SidebarProps> = ({ rootPath, onFileSelect, currentFile, onOpenSettings }) => {
  const [rootFiles, setRootFiles] = useState<FileEntry[]>([]);
  const [width, setWidth] = useState(260);
  const [isResizing, setIsResizing] = useState(false);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const sidebarRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (rootPath) {
      invoke<FileEntry[]>('list_files', { path: rootPath })
        .then(setRootFiles)
        .catch(console.error);
    }
  }, [rootPath]);

  const startResizing = () => {
    setIsResizing(true);
  };

  const stopResizing = () => {
    setIsResizing(false);
  };

  const resize = (mouseMoveEvent: MouseEvent) => {
    if (isResizing) {
      const newWidth = mouseMoveEvent.clientX;
      if (newWidth > 150 && newWidth < 600) {
        setWidth(newWidth);
      }
    }
  };

  useEffect(() => {
    window.addEventListener("mousemove", resize);
    window.addEventListener("mouseup", stopResizing);
    return () => {
      window.removeEventListener("mousemove", resize);
      window.removeEventListener("mouseup", stopResizing);
    };
  }, [isResizing]);

  if (isCollapsed) {
    return (
      <div className="w-12 h-full bg-gray-50 border-r border-gray-200 flex flex-col items-center py-4">
        <button 
          onClick={() => setIsCollapsed(false)}
          className="p-2 hover:bg-gray-200 rounded-md text-gray-600 mb-4"
          title="Expand Sidebar"
        >
          <Menu size={20} />
        </button>
      </div>
    );
  }

  return (
    <div 
      ref={sidebarRef}
      className="h-full bg-gray-50 flex flex-col relative group"
      style={{ width: width }}
    >
      {/* Header */}
      <div className="h-10 flex items-center justify-between px-3 border-b border-gray-200 shrink-0">
        <span className="text-xs font-semibold text-gray-500 uppercase tracking-wider truncate" title={rootPath}>
          {rootPath ? rootPath.split('/').pop() : 'Files'}
        </span>
        <div className="flex items-center">
            <button 
                onClick={onOpenSettings}
                className="p-1 text-gray-400 hover:text-gray-700 hover:bg-gray-200 rounded-md transition-colors"
                title="Settings"
            >
                <Settings size={14} />
            </button>
            <button 
                onClick={() => setIsCollapsed(true)}
                className="p-1 text-gray-400 hover:text-gray-700 hover:bg-gray-200 rounded-md transition-colors ml-1"
                title="Collapse"
            >
                <ChevronLeft size={14} />
            </button>
        </div>
      </div>

      {/* File Tree */}
      <div className="flex-1 overflow-y-auto py-2 custom-scrollbar">
        {rootFiles.map((file) => (
          <FileTreeItem
            key={file.path}
            entry={file}
            depth={0}
            onSelect={onFileSelect}
            currentFile={currentFile}
          />
        ))}
      </div>

      {/* Resize Handle */}
      <div
        className="absolute top-0 right-0 w-1 h-full cursor-col-resize hover:bg-blue-400 transition-colors z-20"
        onMouseDown={startResizing}
      />
      {/* Visual border */}
      <div className="absolute top-0 right-0 w-px h-full bg-gray-200 pointer-events-none" />
    </div>
  );
};
