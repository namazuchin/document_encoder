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
  custom_prompt?: string;
}

interface PromptPreset {
  id: string;
  name: string;
  prompt: string;
  is_default?: boolean;
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
  const [saveDirectory, setSaveDirectory] = useState<string>("");
  const [currentPrompt, setCurrentPrompt] = useState<string>("");
  const [promptPresets, setPromptPresets] = useState<PromptPreset[]>([]);
  const [showPromptSettings, setShowPromptSettings] = useState(false);
  const [editingPreset, setEditingPreset] = useState<PromptPreset | null>(null);
  const [showEditModal, setShowEditModal] = useState(false);
  const [newPresetName, setNewPresetName] = useState("");
  const [newPresetPrompt, setNewPresetPrompt] = useState("");
  const [isDeleting, setIsDeleting] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [deleteTargetId, setDeleteTargetId] = useState<string | null>(null);

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

  const handleSelectSaveDirectory = async () => {
    try {
      const directory = await invoke<string | null>("select_save_directory");
      if (directory) {
        setSaveDirectory(directory);
        addLog(`✅ 保存先ディレクトリを選択: ${directory}`);
      }
    } catch (error) {
      addLog(`❌ 保存先ディレクトリ選択エラー: ${error}`);
      console.error("Error selecting save directory:", error);
    }
  };

  const generateFilename = (files: VideoFile[]): string => {
    if (files.length === 0) return "document.md";
    
    const firstFile = files[0];
    const filename = firstFile.name;
    // 拡張子を除去してMarkdownファイル名を生成
    const nameWithoutExt = filename.replace(/\.[^/.]+$/, "");
    return `${nameWithoutExt}.md`;
  };

  const handleSaveDocument = async () => {
    if (!generatedDocument) {
      addLog("❌ 保存するドキュメントがありません");
      return;
    }

    if (!saveDirectory) {
      addLog("❌ 保存先ディレクトリが選択されていません");
      return;
    }

    try {
      const filename = generateFilename(selectedFiles);
      const savedPath = await invoke<string>("save_document_to_file", {
        content: generatedDocument,
        savePath: saveDirectory,
        filename: filename
      });
      addLog(`✅ ドキュメントを保存しました: ${savedPath}`);
    } catch (error) {
      addLog(`❌ ドキュメント保存エラー: ${error}`);
      console.error("Error saving document:", error);
    }
  };

  useEffect(() => {
    loadSettings();
    loadPromptPresets();
    
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

    // 保存先ディレクトリを選択
    let currentSaveDirectory = saveDirectory;
    if (!currentSaveDirectory) {
      addLog("📁 保存先ディレクトリを選択してください");
      try {
        const directory = await invoke<string | null>("select_save_directory");
        if (directory) {
          currentSaveDirectory = directory;
          setSaveDirectory(directory);
          addLog(`✅ 保存先ディレクトリを選択: ${directory}`);
        } else {
          addLog("❌ 保存先ディレクトリが選択されていないため処理を中止します");
          return;
        }
      } catch (error) {
        addLog(`❌ 保存先ディレクトリ選択エラー: ${error}`);
        return;
      }
    }

    const filename = generateFilename(selectedFiles);
    addLog(`📝 生成予定ファイル名: ${filename}`);
    addLog(`📁 保存先: ${currentSaveDirectory}`);

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
        settings: {
          ...settings,
          custom_prompt: currentPrompt || undefined
        }
      });
      addLog("✅ Document generation completed successfully");
      addLog(`📄 Generated document length: ${result.length}`);
      setGeneratedDocument(result);
      setProgressMessage("処理が完了しました！");

      // 自動保存
      try {
        const savedPath = await invoke<string>("save_document_to_file", {
          content: result,
          savePath: currentSaveDirectory,
          filename: filename
        });
        addLog(`💾 ドキュメントを自動保存しました: ${savedPath}`);
      } catch (saveError) {
        addLog(`❌ 自動保存に失敗しました: ${saveError}`);
      }
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

  const loadPromptPresets = async () => {
    addLog("📖 Loading prompt presets...");
    try {
      const presets = await invoke<PromptPreset[]>("load_prompt_presets");
      setPromptPresets(presets);
      addLog(`✅ Loaded ${presets.length} prompt presets`);
    } catch (error) {
      addLog(`❌ Error loading prompt presets: ${error}`);
      console.error("Error loading prompt presets:", error);
    }
  };

  const handlePromptPresetSelect = (presetId: string) => {
    const preset = promptPresets.find(p => p.id === presetId);
    if (preset) {
      setCurrentPrompt(preset.prompt);
      addLog(`✅ プロンプトプリセットを選択: ${preset.name}`);
    }
  };

  const handlePresetEdit = (preset: PromptPreset) => {
    if (preset.is_default) {
      alert('デフォルトプリセットは編集できません。');
      addLog(`❌ デフォルトプリセットの編集を拒否: ${preset.name}`);
      return;
    }
    setEditingPreset(preset);
    setNewPresetName(preset.name);
    setNewPresetPrompt(preset.prompt);
    setShowEditModal(true);
    addLog(`🖊️ プリセット編集開始: ${preset.name}`);
  };

  const handlePresetDeleteRequest = (presetId: string) => {
    addLog(`🔍 削除ボタンクリック: プリセットID=${presetId}`);
    
    // 削除処理中の場合は無視
    if (isDeleting || showDeleteConfirm) {
      addLog(`⚠️ 削除処理中または確認中のため無視: ${presetId}`);
      return;
    }
    
    const preset = promptPresets.find(p => p.id === presetId);
    addLog(`🔍 対象プリセット検索結果: ${preset ? `名前=${preset.name}, デフォルト=${preset.is_default}` : '見つからない'}`);
    
    if (preset?.is_default) {
      alert('デフォルトプリセットは削除できません。');
      addLog(`❌ デフォルトプリセットの削除を拒否: ${preset.name}`);
      return;
    }
    
    // 削除確認モーダルを表示
    addLog(`❓ 削除確認モーダルを表示: ${preset?.name || presetId}`);
    setDeleteTargetId(presetId);
    setShowDeleteConfirm(true);
  };

  const handleConfirmDelete = async () => {
    if (!deleteTargetId) {
      addLog(`❌ 削除対象IDが設定されていません`);
      return;
    }

    addLog(`✅ ユーザーが削除を確認しました: ${deleteTargetId}`);
    setShowDeleteConfirm(false);
    setIsDeleting(true);
    
    const preset = promptPresets.find(p => p.id === deleteTargetId);
    
    try {
      addLog(`🗑️ プリセット削除を実行開始: ${preset?.name || deleteTargetId}`);
      
      // Filter out only the target preset (default presets are protected by frontend checks)
      const updatedPresets = promptPresets.filter(p => p.id !== deleteTargetId);
      addLog(`📊 削除後のプリセット数: ${updatedPresets.length} (削除前: ${promptPresets.length})`);
      
      await invoke("save_prompt_presets", { presets: updatedPresets });
      addLog(`💾 バックエンド保存完了`);
      
      setPromptPresets(updatedPresets);
      addLog(`🔄 フロントエンド状態更新完了`);
      
      addLog(`✅ プリセットを削除しました: ${preset?.name || deleteTargetId}`);
    } catch (error) {
      addLog(`❌ プリセット削除エラー: ${error}`);
      console.error("Error deleting preset:", error);
    } finally {
      // 削除処理完了フラグ解除
      setIsDeleting(false);
      setDeleteTargetId(null);
      addLog(`🏁 削除処理完了`);
    }
  };

  const handleCancelDelete = () => {
    const preset = promptPresets.find(p => p.id === deleteTargetId);
    addLog(`❌ ユーザーが削除をキャンセルしました: ${preset?.name || deleteTargetId}`);
    setShowDeleteConfirm(false);
    setDeleteTargetId(null);
  };

  const handleNewPreset = () => {
    setEditingPreset(null);
    setNewPresetName("");
    setNewPresetPrompt("");
    setShowEditModal(true);
    addLog("➕ 新規プリセット作成を開始");
  };

  const handleSavePreset = async () => {
    if (!newPresetName.trim() || !newPresetPrompt.trim()) {
      alert("プリセット名とプロンプトの両方を入力してください。");
      return;
    }

    try {
      let updatedPresets;
      
      if (editingPreset) {
        // 編集モード
        updatedPresets = promptPresets.map(p => 
          p.id === editingPreset.id 
            ? { ...p, name: newPresetName, prompt: newPresetPrompt }
            : p
        );
        addLog(`✅ プリセットを更新: ${newPresetName}`);
      } else {
        // 新規作成モード
        const newPreset: PromptPreset = {
          id: `preset_${Date.now()}`,
          name: newPresetName,
          prompt: newPresetPrompt,
          is_default: false
        };
        updatedPresets = [...promptPresets, newPreset];
        addLog(`✅ 新規プリセットを作成: ${newPresetName}`);
      }

      await invoke("save_prompt_presets", { presets: updatedPresets });
      setPromptPresets(updatedPresets);
      setShowEditModal(false);
      setEditingPreset(null);
      setNewPresetName("");
      setNewPresetPrompt("");
    } catch (error) {
      addLog(`❌ プリセット保存エラー: ${error}`);
      console.error("Error saving preset:", error);
    }
  };

  const handleImportXML = async () => {
    try {
      const importedPresets = await invoke<PromptPreset[]>("import_prompt_presets_from_file");
      setPromptPresets(importedPresets);
      addLog(`✅ XMLファイルから${importedPresets.length}個のプリセットを読み込みました`);
    } catch (error) {
      addLog(`❌ XMLファイル読み込みエラー: ${error}`);
      console.error("Error importing XML:", error);
    }
  };

  const handleExportXML = async () => {
    try {
      await invoke("export_prompt_presets_to_file", { presets: promptPresets });
      addLog(`✅ ${promptPresets.length}個のプリセットをXMLファイルに出力しました`);
    } catch (error) {
      addLog(`❌ XMLファイル出力エラー: ${error}`);
      console.error("Error exporting XML:", error);
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
        <h1>API設定</h1>
        <div className="settings-form">
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

  if (showPromptSettings) {
    return (
      <main className="container">
        <h1>プロンプト設定</h1>
        <div className="settings-form">
          <div className="form-group">
            <label>プロンプトプリセット管理</label>
            <div className="preset-list">
              {promptPresets.map(preset => (
                <div key={preset.id} className={`preset-item ${preset.is_default ? 'preset-default' : ''}`}>
                  <div className="preset-info">
                    <span className="preset-name">
                      {preset.name}
                      {preset.is_default && <span className="default-badge">デフォルト</span>}
                    </span>
                    <span className="preset-preview">{preset.prompt.substring(0, 50)}...</span>
                  </div>
                  <div className="preset-actions">
                    {!preset.is_default && (
                      <>
                        <button onClick={(e) => { e.stopPropagation(); handlePresetEdit(preset); }}>編集</button>
                        <button 
                          onClick={(e) => { e.stopPropagation(); handlePresetDeleteRequest(preset.id); }}
                          disabled={isDeleting || showDeleteConfirm}
                          className={isDeleting ? 'deleting' : ''}
                        >
                          {isDeleting ? '削除中...' : '削除'}
                        </button>
                      </>
                    )}
                  </div>
                </div>
              ))}
            </div>
            <div className="button-group">
              <button onClick={handleNewPreset}>新規プリセット作成</button>
              <button onClick={handleImportXML}>XMLファイルから読み込み</button>
              <button onClick={handleExportXML}>XMLファイルにエクスポート</button>
            </div>
          </div>
          
          <div className="button-group">
            <button onClick={() => setShowPromptSettings(false)}>閉じる</button>
          </div>
        </div>

        {showEditModal && (
          <div className="modal-overlay">
            <div className="modal-content">
              <h3>{editingPreset ? 'プリセット編集' : '新規プリセット作成'}</h3>
              <div className="form-group">
                <label htmlFor="presetName">プリセット名:</label>
                <input
                  type="text"
                  id="presetName"
                  value={newPresetName}
                  onChange={(e) => setNewPresetName(e.target.value)}
                  placeholder="プリセット名を入力..."
                />
              </div>
              <div className="form-group">
                <label htmlFor="presetPrompt">プロンプト:</label>
                <textarea
                  id="presetPrompt"
                  value={newPresetPrompt}
                  onChange={(e) => setNewPresetPrompt(e.target.value)}
                  placeholder="プロンプトを入力..."
                  rows={6}
                />
              </div>
              <div className="modal-buttons">
                <button onClick={handleSavePreset}>
                  {editingPreset ? '更新' : '作成'}
                </button>
                <button onClick={() => setShowEditModal(false)}>キャンセル</button>
              </div>
            </div>
          </div>
        )}

        {showDeleteConfirm && (
          <div className="modal-overlay">
            <div className="modal-content delete-confirm">
              <h3>プリセット削除の確認</h3>
              <p>
                「{promptPresets.find(p => p.id === deleteTargetId)?.name || 'このプリセット'}」を削除しますか？
              </p>
              <p className="warning-text">
                ⚠️ この操作は取り消せません。
              </p>
              <div className="modal-buttons">
                <button className="delete-confirm-btn" onClick={handleConfirmDelete}>
                  削除する
                </button>
                <button onClick={handleCancelDelete}>キャンセル</button>
              </div>
            </div>
          </div>
        )}
      </main>
    );
  }

  return (
    <main className="container">
      <header className="header">
        <h1>Document Encoder</h1>
        <div className="header-buttons">
          <button className="settings-btn" onClick={() => setShowSettings(true)}>
            API設定
          </button>
          <button className="settings-btn" onClick={() => setShowPromptSettings(true)}>
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
              onChange={async (e) => {
                const newSettings = { ...settings, mode: e.target.value as DocumentMode };
                setSettings(newSettings);
                try {
                  await invoke("save_settings", { settings: newSettings });
                  addLog(`✅ ドキュメントモードを変更: ${e.target.value === "manual" ? "マニュアル作成" : "仕様書作成"}`);
                } catch (error) {
                  addLog(`❌ 設定保存エラー: ${error}`);
                }
              }}
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
              onChange={async (e) => {
                const newSettings = { ...settings, language: e.target.value };
                setSettings(newSettings);
                try {
                  await invoke("save_settings", { settings: newSettings });
                  addLog(`✅ 出力言語を変更: ${e.target.value === "japanese" ? "日本語" : "English"}`);
                } catch (error) {
                  addLog(`❌ 設定保存エラー: ${error}`);
                }
              }}
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
              onChange={(e) => handlePromptPresetSelect(e.target.value)}
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
              onChange={(e) => setCurrentPrompt(e.target.value)}
              placeholder="プロンプトを入力してください..."
              rows={4}
            />
          </div>
        </div>
      </div>

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


      <div className="save-directory-section">
        <h2>保存設定</h2>
        <button className="directory-select-btn" onClick={handleSelectSaveDirectory}>
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
          <div className="result-header">
            <h2>生成結果</h2>
            <button 
              className="save-btn"
              onClick={handleSaveDocument}
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

export default App;
