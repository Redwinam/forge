import React, { useState } from 'react';
import { X } from 'lucide-react';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  rootPath: string;
  onSaveRootPath: (path: string) => void;
  envPath: string;
  onSaveEnvPath: (path: string) => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  onClose,
  rootPath,
  onSaveRootPath,
  envPath,
  onSaveEnvPath,
}) => {
  const [localRootPath, setLocalRootPath] = useState(rootPath);
  const [localEnvPath, setLocalEnvPath] = useState(envPath);

  if (!isOpen) return null;

  const handleSave = () => {
    onSaveRootPath(localRootPath);
    onSaveEnvPath(localEnvPath);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-white rounded-lg shadow-xl w-[500px] max-w-full m-4 overflow-hidden">
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
          <h2 className="text-lg font-semibold text-gray-900">Settings</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-gray-600">
            <X size={20} />
          </button>
        </div>
        
        <div className="p-6 space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Documents Root Path
            </label>
            <input
              type="text"
              value={localRootPath}
              onChange={(e) => setLocalRootPath(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm"
              placeholder="/path/to/docs"
            />
            <p className="mt-1 text-xs text-gray-500">
              Absolute path to your markdown files directory.
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              .env File Path (Tencent COS)
            </label>
            <input
              type="text"
              value={localEnvPath}
              onChange={(e) => setLocalEnvPath(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm"
              placeholder="/path/to/.env"
            />
            <p className="mt-1 text-xs text-gray-500">
              Path to .env file containing TENCENT_SECRET_ID, TENCENT_SECRET_KEY, etc.
            </p>
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 px-6 py-4 bg-gray-50 border-t border-gray-200">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
          >
            Save Changes
          </button>
        </div>
      </div>
    </div>
  );
};
