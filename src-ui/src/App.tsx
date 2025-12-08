import { useState } from 'react';
import { Sidebar } from './components/Sidebar';
import { Editor } from './components/Editor';
import { SettingsModal } from './components/SettingsModal';
import { invoke } from '@tauri-apps/api/core';

function App() {
  const [currentPath, setCurrentPath] = useState<string | null>(null);
  const [currentContent, setCurrentContent] = useState<string>('');
  
  // Settings State
  const [rootPath, setRootPath] = useState<string>(() => {
    return localStorage.getItem('forge_root_path') || '/Users/redwinam/Developer/notes/Press/docs';
  });
  const [envPath, setEnvPath] = useState<string>(() => {
    return localStorage.getItem('forge_env_path') || '/Users/redwinam/Developer/notes/Press/.env';
  });
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  const handleSaveRootPath = (path: string) => {
    setRootPath(path);
    localStorage.setItem('forge_root_path', path);
  };

  const handleSaveEnvPath = (path: string) => {
    setEnvPath(path);
    localStorage.setItem('forge_env_path', path);
  };

  const handleFileSelect = async (path: string) => {
    try {
      const content = await invoke<string>('read_file', { path });
      setCurrentPath(path);
      setCurrentContent(content);
    } catch (e) {
      console.error("Failed to read file", e);
      alert("Failed to read file: " + e);
    }
  };

  const handleSave = async (content: string) => {
    if (!currentPath) return;
    try {
      await invoke('write_file', { path: currentPath, content });
      console.log("Saved!");
    } catch (e) {
      console.error("Failed to save", e);
      alert("Failed to save: " + e);
    }
  };

  return (
    <div className="flex h-screen w-screen bg-white text-black overflow-hidden">
      <Sidebar 
        rootPath={rootPath} 
        onFileSelect={handleFileSelect} 
        currentFile={currentPath}
        onOpenSettings={() => setIsSettingsOpen(true)}
      />
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        {currentPath ? (
          <Editor 
            key={currentPath} // Force re-mount when file changes to reset history etc.
            content={currentContent} 
            filePath={currentPath} 
            onSave={handleSave} 
            className="flex-1"
            envPath={envPath}
          />
        ) : (
          <div className="flex items-center justify-center h-full text-gray-400">
            Select a file to edit
          </div>
        )}
      </div>
      
      <SettingsModal 
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        rootPath={rootPath}
        onSaveRootPath={handleSaveRootPath}
        envPath={envPath}
        onSaveEnvPath={handleSaveEnvPath}
      />
    </div>
  );
}

export default App;
