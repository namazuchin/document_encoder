import React from 'react';
import { AppSettings } from '../types';

interface ApiSettingsProps {
  settings: AppSettings;
  onUpdateSettings: (settings: AppSettings) => void;
  onClose: () => void;
  onSave: () => void;
}

export default function ApiSettings({ settings, onUpdateSettings, onClose, onSave }: ApiSettingsProps) {
  return (
    <main className="container">
      <h1>API設定</h1>
      <div className="settings-form">
        <div className="form-group">
          <label htmlFor="apiKey">Gemini API Key:</label>
          <input
            type="password"
            id="apiKey"
            value={settings.gemini_api_key}
            onChange={(e) => onUpdateSettings({ ...settings, gemini_api_key: e.target.value })}
            placeholder="API keyを入力してください"
          />
        </div>
        
        <div className="button-group">
          <button onClick={onSave}>保存</button>
          <button onClick={onClose}>キャンセル</button>
        </div>
      </div>
    </main>
  );
}