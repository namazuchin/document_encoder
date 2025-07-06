import React, { useRef } from 'react';
import { VideoFile, AppSettings, PromptPreset } from '../types';

interface MainDashboardProps {
  settings: AppSettings;
  onUpdateSettings: (settings: AppSettings) => void;
  selectedFiles: VideoFile[];
  onFileSelect: () => void;
  onRemoveFile: (index: number) => void;
  currentPrompt: string;
  onPromptChange: (prompt: string) => void;
  promptPresets: PromptPreset[];
  selectedPresetId: string;
  onPromptPresetSelect: (presetId: string) => void;
  saveDirectory: string;
  onSelectSaveDirectory: () => void;
  onGenerateDocument: () => void;
  isProcessing: boolean;
  progressMessage: string;
  progressStep: number;
  totalSteps: number;
  logs: string[];
  showLogs: boolean;
  onToggleLogs: () => void;
  onClearLogs: () => void;
  generatedDocument: string;
  onShowSettings: () => void;
  onShowPromptSettings: () => void;
  generateFilename: (files: VideoFile[]) => string;
}

export default function MainDashboard({
  settings,
  onUpdateSettings,
  selectedFiles,
  onFileSelect,
  onRemoveFile,
  currentPrompt,
  onPromptChange,
  promptPresets,
  selectedPresetId,
  onPromptPresetSelect,
  saveDirectory,
  onSelectSaveDirectory,
  onGenerateDocument,
  isProcessing,
  progressMessage,
  progressStep,
  totalSteps,
  logs,
  showLogs,
  onToggleLogs,
  onClearLogs,
  generatedDocument,
  onShowSettings,
  onShowPromptSettings,
  generateFilename
}: MainDashboardProps) {
  const logContainerRef = useRef<HTMLDivElement>(null);

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  const handleLanguageChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newSettings = { ...settings, language: e.target.value };
    onUpdateSettings(newSettings);
  };

  return (
    <main className="container">
      <header className="header">
        <h1>Document Encoder</h1>
        <div className="header-buttons">
          <button className="settings-btn" onClick={onShowSettings}>
            API設定
          </button>
          <button className="settings-btn" onClick={onShowPromptSettings}>
            プロンプト設定
          </button>
        </div>
      </header>

      <div className="main-content">
        <div className="left-panel">
          <div className="prompt-section">
            <h2>プロンプト設定</h2>
            <div className="prompt-controls">
              <div className="prompt-row">
                <div className="setting-group">
                  <label htmlFor="presetSelect">プリセット選択:</label>
                  <select 
                    id="presetSelect"
                    onChange={(e) => onPromptPresetSelect(e.target.value)}
                    value={selectedPresetId}
                  >
                    <option value="">プリセットを選択...</option>
                    {promptPresets.map(preset => (
                      <option key={preset.id} value={preset.id}>{preset.name}</option>
                    ))}
                  </select>
                </div>
                <div className="setting-group">
                  <label htmlFor="language">ドキュメント出力言語:</label>
                  <select 
                    id="language"
                    value={settings.language}
                    onChange={handleLanguageChange}
                  >
                    <option value="japanese">日本語</option>
                    <option value="english">English</option>
                  </select>
                </div>
              </div>
              <div className="prompt-editor">
                <label htmlFor="promptText">現在のプロンプト:</label>
                <textarea
                  id="promptText"
                  value={currentPrompt}
                  onChange={(e) => onPromptChange(e.target.value)}
                  placeholder="プロンプトを入力してください..."
                  rows={4}
                />
              </div>
            </div>
          </div>

          <div className="file-selection">
            <h2>動画ファイル選択</h2>
            <button className="file-select-btn" onClick={onFileSelect}>
              ファイルを選択
            </button>
            
            {selectedFiles.length > 0 && (
              <div className="file-list">
                <h3>選択されたファイル ({selectedFiles.length}件)</h3>
                <div className="file-list-container">
                  {selectedFiles.map((file, index) => (
                    <div key={index} className="file-item">
                      <span className="file-name">{file.name}</span>
                      <span className="file-size">({formatFileSize(file.size)})</span>
                      <button 
                        className="remove-btn"
                        onClick={() => onRemoveFile(index)}
                      >
                        削除
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>

          <div className="save-directory-section">
            <h2>保存設定</h2>
            <button className="directory-select-btn" onClick={onSelectSaveDirectory}>
              保存先を変更
            </button>
            <p className="directory-preview">
              保存先: {saveDirectory || "未選択"}
            </p>
            <p className="filename-preview">
              生成ファイル名: {selectedFiles.length > 0 ? generateFilename(selectedFiles) : "ファイルが選択されていません"}
            </p>
          </div>

          <div className="generate-section">
            <button 
              className="generate-btn"
              onClick={onGenerateDocument}
              disabled={isProcessing || selectedFiles.length === 0}
            >
              {isProcessing ? "処理中..." : "ドキュメント生成"}
            </button>
          </div>
        </div>

        <div className="right-panel">
          <div className="progress-section">
            <h2>処理状況</h2>
            <div className="progress-message">{progressMessage || "待機中..."}</div>
            <div className="progress-bar-container">
              <div className="progress-bar">
                <div 
                  className="progress-bar-fill"
                  style={{ width: `${totalSteps > 0 ? (progressStep / totalSteps) * 100 : 0}%` }}
                ></div>
              </div>
              <div className="progress-text">
                {progressStep} / {totalSteps}
              </div>
            </div>
            
            <div className="log-section">
              <div className="log-header">
                <span>処理ログ ({logs.length}件)</span>
                <div className="log-buttons">
                  <button 
                    className="log-toggle-btn"
                    onClick={onToggleLogs}
                  >
                    {showLogs ? '非表示' : '表示'}
                  </button>
                  {logs.length > 0 && (
                    <button 
                      className="log-clear-btn"
                      onClick={onClearLogs}
                    >
                      クリア
                    </button>
                  )}
                </div>
              </div>
              <div className="log-container" ref={logContainerRef}>
                {logs.map((log, index) => (
                  <div key={index} className="log-entry">
                    {log}
                  </div>
                ))}
                {logs.length === 0 && (
                  <div className="log-entry log-empty">
                    ログはまだありません
                  </div>
                )}
              </div>
            </div>
          </div>

          <div className="result-section">
            <h2>生成結果</h2>
            <div className="document-content">
              <pre>{generatedDocument || "まだドキュメントが生成されていません"}</pre>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
