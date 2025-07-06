import React, { useRef } from 'react';
import { DocumentMode, VideoFile, AppSettings, PromptPreset } from '../types';

interface MainDashboardProps {
  settings: AppSettings;
  onUpdateSettings: (settings: AppSettings) => void;
  selectedFiles: VideoFile[];
  onFileSelect: () => void;
  onRemoveFile: (index: number) => void;
  currentPrompt: string;
  onPromptChange: (prompt: string) => void;
  promptPresets: PromptPreset[];
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
  onSaveDocument: () => void;
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
  onSaveDocument,
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

  const handleModeChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newSettings = { ...settings, mode: e.target.value as DocumentMode };
    onUpdateSettings(newSettings);
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

      <div className="mode-language-section">
        <h2>ドキュメント設定</h2>
        <div className="settings-row">
          <div className="setting-group">
            <label htmlFor="mode">ドキュメントモード:</label>
            <select 
              id="mode"
              value={settings.mode}
              onChange={handleModeChange}
            >
              <option value="manual">マニュアル作成モード</option>
              <option value="specification">仕様書作成モード</option>
            </select>
          </div>
          
          <div className="setting-group">
            <label htmlFor="language">出力言語:</label>
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
      </div>

      <div className="prompt-section">
        <h2>プロンプト設定</h2>
        <div className="prompt-controls">
          <div className="preset-selector">
            <label htmlFor="presetSelect">プリセット選択:</label>
            <select 
              id="presetSelect"
              onChange={(e) => onPromptPresetSelect(e.target.value)}
              value=""
            >
              <option value="">プリセットを選択...</option>
              {promptPresets.map(preset => (
                <option key={preset.id} value={preset.id}>{preset.name}</option>
              ))}
            </select>
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
        )}
      </div>

      <div className="save-directory-section">
        <h2>保存設定</h2>
        <button className="directory-select-btn" onClick={onSelectSaveDirectory}>
          {saveDirectory ? "保存先を変更" : "保存先ディレクトリを選択"}
        </button>
        {saveDirectory && (
          <p className="directory-preview">
            保存先: {saveDirectory}
          </p>
        )}
        {selectedFiles.length > 0 && (
          <p className="filename-preview">
            生成ファイル名: {generateFilename(selectedFiles)}
          </p>
        )}
      </div>

      <div className="generate-section">
        <button 
          className="generate-btn"
          onClick={onGenerateDocument}
          disabled={isProcessing || selectedFiles.length === 0}
        >
          {isProcessing ? "処理中..." : "ドキュメント生成"}
        </button>
        
        {isProcessing && (
          <div className="progress-section">
            <div className="progress-message">{progressMessage}</div>
            {totalSteps > 0 && (
              <div className="progress-bar-container">
                <div className="progress-bar">
                  <div 
                    className="progress-bar-fill"
                    style={{ width: `${(progressStep / totalSteps) * 100}%` }}
                  ></div>
                </div>
                <div className="progress-text">
                  {progressStep} / {totalSteps}
                </div>
              </div>
            )}
            
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
              {showLogs && (
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
              )}
            </div>
          </div>
        )}
      </div>

      {generatedDocument && (
        <div className="result-section">
          <div className="result-header">
            <h2>生成結果</h2>
            <button 
              className="save-btn"
              onClick={onSaveDocument}
              disabled={!saveDirectory}
            >
              再保存
            </button>
          </div>
          <div className="document-content">
            <pre>{generatedDocument}</pre>
          </div>
        </div>
      )}
    </main>
  );
}