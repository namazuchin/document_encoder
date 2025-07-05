import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type DocumentMode = "manual" | "specification";

interface VideoFile {
  path: string;
  name: string;
  size: number;
}

interface AppSettings {
  mode: DocumentMode;
  gemini_api_key: string;
  language: string;
}

interface ProgressUpdate {
  message: string;
  step: number;
  total_steps: number;
}

function App() {
  const [selectedFiles, setSelectedFiles] = useState<VideoFile[]>([]);
  const [settings, setSettings] = useState<AppSettings>({
    mode: "manual",
    gemini_api_key: "",
    language: "japanese"
  });
  const [isProcessing, setIsProcessing] = useState(false);
  const [generatedDocument, setGeneratedDocument] = useState("");
  const [showSettings, setShowSettings] = useState(false);
  const [progressMessage, setProgressMessage] = useState("");
  const [progressStep, setProgressStep] = useState(0);
  const [totalSteps, setTotalSteps] = useState(0);
  const [logs, setLogs] = useState<string[]>([]);
  const [showLogs, setShowLogs] = useState(false);
  const logContainerRef = useRef<HTMLDivElement>(null);

  const addLog = (message: string) => {
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = `[${timestamp}] ${message}`;
    setLogs(prev => [...prev, logEntry]);
    console.log(logEntry);
    
    // Auto-scroll to bottom when new log is added
    setTimeout(() => {
      if (logContainerRef.current) {
        logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
      }
    }, 100);
  };

  const clearLogs = () => {
    setLogs([]);
  };

  useEffect(() => {
    loadSettings();
    
    // Progress update listener
    addLog("🎧 Setting up progress update listener...");
    const unsubscribe = listen<ProgressUpdate>("progress_update", (event) => {
      const { message, step, total_steps } = event.payload;
      addLog(`📊 [FRONTEND] Received progress update: ${step}/${total_steps} - ${message}`);
      setProgressMessage(message);
      setProgressStep(step);
      setTotalSteps(total_steps);
    });

    return () => {
      unsubscribe.then(f => f());
    };
  }, []);

  const handleFileSelect = async () => {
    addLog("📁 Starting file selection...");
    try {
      const files = await invoke<VideoFile[]>("select_video_files");
      addLog(`✅ Selected ${files.length} files: ${files.map(f => f.name).join(", ")}`);
      setSelectedFiles(files);
    } catch (error) {
      addLog(`❌ Error selecting files: ${error}`);
      addLog(`📊 File selection error details: ${JSON.stringify(error, null, 2)}`);
      console.error("Error selecting files:", error);
    }
  };

  const handleRemoveFile = (index: number) => {
    setSelectedFiles(files => files.filter((_, i) => i !== index));
  };

  const handleGenerateDocument = async () => {
    addLog("🚀 Starting document generation process");
    
    if (selectedFiles.length === 0) {
      addLog("❌ No video files selected");
      return;
    }
    
    if (!settings.gemini_api_key) {
      addLog("❌ Gemini API key is not set");
      return;
    }

    addLog(`📁 Processing ${selectedFiles.length} files: ${selectedFiles.map(f => f.name).join(", ")}`);
    addLog(`⚙️ Settings: mode=${settings.mode}, language=${settings.language}`);

    setIsProcessing(true);
    setProgressMessage("処理を開始しています...");
    setProgressStep(0);
    setTotalSteps(0);
    setShowLogs(true); // 処理開始時にログを表示
    
    try {
      addLog("📤 Sending request to backend...");
      const result = await invoke<string>("generate_document", {
        files: selectedFiles,
        settings: settings
      });
      addLog("✅ Document generation completed successfully");
      addLog(`📄 Generated document length: ${result.length}`);
      setGeneratedDocument(result);
      setProgressMessage("処理が完了しました！");
    } catch (error) {
      addLog(`❌ Error generating document: ${error}`);
      addLog(`📊 Error details: ${JSON.stringify(error, null, 2)}`);
      setProgressMessage("エラーが発生しました。");
      console.error("Error generating document:", error);
    } finally {
      setIsProcessing(false);
      addLog("🏁 Document generation process finished");
    }
  };

  const handleSaveSettings = async () => {
    addLog(`💾 Saving settings: mode=${settings.mode}, language=${settings.language}`);
    try {
      await invoke("save_settings", { settings });
      addLog("✅ Settings saved successfully");
      setShowSettings(false);
    } catch (error) {
      addLog(`❌ Error saving settings: ${error}`);
      addLog(`📊 Settings save error details: ${JSON.stringify(error, null, 2)}`);
      console.error("Error saving settings:", error);
    }
  };

  const loadSettings = async () => {
    addLog("📖 Loading settings...");
    try {
      const savedSettings = await invoke<AppSettings | null>("load_settings");
      if (savedSettings) {
        addLog(`✅ Settings loaded successfully: mode=${savedSettings.mode}, language=${savedSettings.language}`);
        setSettings(savedSettings);
      } else {
        addLog("ℹ️ No saved settings found, using defaults");
      }
    } catch (error) {
      addLog(`❌ Error loading settings: ${error}`);
      addLog(`📊 Settings load error details: ${JSON.stringify(error, null, 2)}`);
      console.error("Error loading settings:", error);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  if (showSettings) {
    return (
      <main className="container">
        <h1>設定</h1>
        <div className="settings-form">
          <div className="form-group">
            <label htmlFor="mode">ドキュメントモード:</label>
            <select 
              id="mode"
              value={settings.mode}
              onChange={(e) => setSettings(prev => ({ ...prev, mode: e.target.value as DocumentMode }))}
            >
              <option value="manual">マニュアル作成モード</option>
              <option value="specification">仕様書作成モード</option>
            </select>
          </div>
          
          <div className="form-group">
            <label htmlFor="language">出力言語:</label>
            <select 
              id="language"
              value={settings.language}
              onChange={(e) => setSettings(prev => ({ ...prev, language: e.target.value }))}
            >
              <option value="japanese">日本語</option>
              <option value="english">English</option>
            </select>
          </div>
          
          <div className="form-group">
            <label htmlFor="apiKey">Gemini API Key:</label>
            <input
              type="password"
              id="apiKey"
              value={settings.gemini_api_key}
              onChange={(e) => setSettings(prev => ({ ...prev, gemini_api_key: e.target.value }))}
              placeholder="API keyを入力してください"
            />
          </div>
          
          <div className="button-group">
            <button onClick={handleSaveSettings}>保存</button>
            <button onClick={() => setShowSettings(false)}>キャンセル</button>
          </div>
        </div>
      </main>
    );
  }

  return (
    <main className="container">
      <header className="header">
        <h1>Document Encoder</h1>
        <button className="settings-btn" onClick={() => setShowSettings(true)}>
          設定
        </button>
      </header>

      <div className="file-selection">
        <h2>動画ファイル選択</h2>
        <button className="file-select-btn" onClick={handleFileSelect}>
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
                  onClick={() => handleRemoveFile(index)}
                >
                  削除
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="mode-indicator">
        <p>現在のモード: {settings.mode === "manual" ? "マニュアル作成" : "仕様書作成"}</p>
        <p>出力言語: {settings.language === "japanese" ? "日本語" : "English"}</p>
      </div>

      <div className="generate-section">
        <button 
          className="generate-btn"
          onClick={handleGenerateDocument}
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
                    onClick={() => setShowLogs(!showLogs)}
                  >
                    {showLogs ? '非表示' : '表示'}
                  </button>
                  {logs.length > 0 && (
                    <button 
                      className="log-clear-btn"
                      onClick={clearLogs}
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
          <h2>生成結果</h2>
          <div className="document-content">
            <pre>{generatedDocument}</pre>
          </div>
        </div>
      )}
    </main>
  );
}

export default App;
