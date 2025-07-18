import React, { useRef, useEffect, useState } from 'react';
import { VideoFile, AppSettings, PromptPreset, VideoQuality, ImageEmbedFrequency } from '../types';
import { 
  FaPlay, 
  FaCog, 
  FaEdit, 
  FaFolder, 
  FaFileVideo, 
  FaTimes, 
  FaEye, 
  FaEyeSlash, 
  FaTrash, 
  FaLanguage,
  FaImage,
  FaVideo
} from 'react-icons/fa';

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
  const [isAutoScroll, setIsAutoScroll] = useState(true);

  // 自動スクロール機能
  useEffect(() => {
    if (isAutoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [logs, isAutoScroll]);

  // スクロールイベントのハンドラー
  const handleScroll = () => {
    if (logContainerRef.current) {
      const { scrollTop, scrollHeight, clientHeight } = logContainerRef.current;
      const isAtBottom = scrollTop + clientHeight >= scrollHeight - 50; // 50px余裕を持たせる
      setIsAutoScroll(isAtBottom);
    }
  };

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

  const handleEmbedImagesChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const newSettings = { ...settings, embed_images: e.target.checked };
    onUpdateSettings(newSettings);
  };

  const handleImageEmbedFrequencyChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newSettings = { ...settings, image_embed_frequency: e.target.value as ImageEmbedFrequency };
    onUpdateSettings(newSettings);
  };

  const handleVideoQualityChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newSettings = { ...settings, video_quality: e.target.value as VideoQuality };
    onUpdateSettings(newSettings);
  };

  return (
    <main className="container">
      <header className="header">
        <h1>Document Encoder</h1>
        <div className="header-buttons">
          <button className="settings-btn" onClick={onShowSettings}>
            <FaCog className="icon" /> 設定
          </button>
          <button className="settings-btn" onClick={onShowPromptSettings}>
            <FaEdit className="icon" /> プロンプト設定
          </button>
        </div>
      </header>

      <div className="main-content">
        {/* 左側ペイン: 設定・操作 */}
        <div className="left-panel">
          <div className="settings-panel">
            <h2>設定</h2>
            
            {/* プロンプト設定 */}
            <div className="setting-section">
              <h3>プロンプト設定</h3>
              <div className="settings-grid">
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
                  <label htmlFor="language"><FaLanguage className="icon" /> 出力言語:</label>
                  <select 
                    id="language"
                    value={settings.language}
                    onChange={handleLanguageChange}
                  >
                    <option value="japanese">日本語</option>
                    <option value="english">English</option>
                  </select>
                </div>
                <div className="setting-group">
                  <label htmlFor="videoQuality"><FaVideo className="icon" /> 動画画質:</label>
                  <select 
                    id="videoQuality"
                    value={settings.video_quality || "NoConversion"}
                    onChange={handleVideoQualityChange}
                  >
                    <option value="NoConversion">変換なし</option>
                    <option value="1080p">1080p</option>
                    <option value="720p">720p</option>
                    <option value="480p">480p</option>
                  </select>
                </div>
                <div className="setting-group">
                  <div className="checkbox-group">
                    <label className="checkbox-label" htmlFor="embedImages">
                      <input
                        type="checkbox"
                        id="embedImages"
                        checked={settings.embed_images || false}
                        onChange={handleEmbedImagesChange}
                      />
                      <span className="checkbox-text">
                        <FaImage className="icon" /> 画像を埋め込む
                      </span>
                    </label>
                  </div>
                  {settings.embed_images && (
                    <div className="frequency-setting">
                      <label htmlFor="imageEmbedFrequency">頻度:</label>
                      <select
                        id="imageEmbedFrequency"
                        value={settings.image_embed_frequency || 'moderate'}
                        onChange={handleImageEmbedFrequencyChange}
                      >
                        <option value="minimal">最小限</option>
                        <option value="moderate">適度</option>
                        <option value="detailed">詳細</option>
                      </select>
                    </div>
                  )}
                </div>
              </div>
              <div className="prompt-editor">
                <label htmlFor="promptText">現在のプロンプト:</label>
                <textarea
                  id="promptText"
                  value={currentPrompt}
                  onChange={(e) => onPromptChange(e.target.value)}
                  placeholder="プロンプトを入力してください..."
                  rows={3}
                />
              </div>
            </div>

            {/* ファイル選択 */}
            <div className="setting-section">
              <h3>動画ファイル選択</h3>
              <button className="file-select-btn" onClick={onFileSelect}>
                <FaFileVideo className="icon" /> ファイルを選択
              </button>
              
              {selectedFiles.length > 0 && (
                <div className="file-list">
                  <div className="file-count">選択されたファイル: {selectedFiles.length}件</div>
                  <div className="file-list-container">
                    {selectedFiles.map((file, index) => (
                      <div key={index} className="file-item">
                        <span className="file-name">{file.name}</span>
                        <span className="file-size">({formatFileSize(file.size)})</span>
                        <button 
                          className="remove-btn"
                          onClick={() => onRemoveFile(index)}
                        >
                          <FaTimes className="icon" />
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* 保存設定 */}
            <div className="setting-section">
              <h3>保存設定</h3>
              <button className="directory-select-btn" onClick={onSelectSaveDirectory}>
                <FaFolder className="icon" /> 保存先を変更
              </button>
              <div className="save-info">
                <div className="directory-preview">
                  保存先: {saveDirectory || "未選択"}
                </div>
                <div className="filename-preview">
                  生成ファイル名: {selectedFiles.length > 0 ? generateFilename(selectedFiles) : "ファイルが選択されていません"}
                </div>
              </div>
            </div>

            {/* ドキュメント生成 */}
            <div className="setting-section">
              <button 
                className="generate-btn"
                onClick={onGenerateDocument}
                disabled={isProcessing || selectedFiles.length === 0}
              >
                <FaPlay className="icon" /> {isProcessing ? "処理中..." : "ドキュメント生成"}
              </button>
            </div>
          </div>
        </div>

        {/* 右側ペイン: 処理状況・結果 */}
        <div className="right-panel">
          {/* 処理状況 */}
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
                    {showLogs ? <><FaEyeSlash className="icon" /> 非表示</> : <><FaEye className="icon" /> 表示</>}
                  </button>
                  {logs.length > 0 && (
                    <button 
                      className="log-clear-btn"
                      onClick={onClearLogs}
                    >
                      <FaTrash className="icon" /> クリア
                    </button>
                  )}
                </div>
              </div>
              {showLogs && (
                <div 
                  className="log-container" 
                  ref={logContainerRef}
                  onScroll={handleScroll}
                >
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

          {/* 生成結果 */}
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
