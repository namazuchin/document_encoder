import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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

  useEffect(() => {
    loadSettings();
  }, []);

  const handleFileSelect = async () => {
    console.log("📁 Starting file selection...");
    try {
      const files = await invoke<VideoFile[]>("select_video_files");
      console.log(`✅ Selected ${files.length} files:`, files.map(f => f.name));
      setSelectedFiles(files);
    } catch (error) {
      console.error("❌ Error selecting files:", error);
      console.error("📊 File selection error details:", JSON.stringify(error, null, 2));
    }
  };

  const handleRemoveFile = (index: number) => {
    setSelectedFiles(files => files.filter((_, i) => i !== index));
  };

  const handleGenerateDocument = async () => {
    console.log("🚀 Starting document generation process");
    
    if (selectedFiles.length === 0) {
      console.error("❌ No video files selected");
      return;
    }
    
    if (!settings.gemini_api_key) {
      console.error("❌ Gemini API key is not set");
      return;
    }

    console.log(`📁 Processing ${selectedFiles.length} files:`, selectedFiles.map(f => f.name));
    console.log("⚙️ Settings:", { mode: settings.mode, language: settings.language });

    setIsProcessing(true);
    try {
      console.log("📤 Sending request to backend...");
      const result = await invoke<string>("generate_document", {
        files: selectedFiles,
        settings: settings
      });
      console.log("✅ Document generation completed successfully");
      console.log("📄 Generated document length:", result.length);
      setGeneratedDocument(result);
    } catch (error) {
      console.error("❌ Error generating document:", error);
      console.error("📊 Error details:", JSON.stringify(error, null, 2));
    } finally {
      setIsProcessing(false);
      console.log("🏁 Document generation process finished");
    }
  };

  const handleSaveSettings = async () => {
    console.log("💾 Saving settings...", { mode: settings.mode, language: settings.language });
    try {
      await invoke("save_settings", { settings });
      console.log("✅ Settings saved successfully");
      setShowSettings(false);
    } catch (error) {
      console.error("❌ Error saving settings:", error);
      console.error("📊 Settings save error details:", JSON.stringify(error, null, 2));
    }
  };

  const loadSettings = async () => {
    console.log("📖 Loading settings...");
    try {
      const savedSettings = await invoke<AppSettings | null>("load_settings");
      if (savedSettings) {
        console.log("✅ Settings loaded successfully:", { mode: savedSettings.mode, language: savedSettings.language });
        setSettings(savedSettings);
      } else {
        console.log("ℹ️ No saved settings found, using defaults");
      }
    } catch (error) {
      console.error("❌ Error loading settings:", error);
      console.error("📊 Settings load error details:", JSON.stringify(error, null, 2));
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
