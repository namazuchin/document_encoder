import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import { VideoFile, AppSettings, PromptPreset, ProgressUpdate, YouTubeVideoInfo } from './types';
import { generateFilename, getDirectoryFromPath } from './utils/fileUtils';
import { useLogger } from './hooks/useLogger';
import Settings from './components/Settings';
import PromptSettings from './components/PromptSettings';
import PresetEditModal from './components/PresetEditModal';
import MainDashboard from './components/MainDashboard';
import LicenseDisplay from './components/LicenseDisplay';

type Page = 'main' | 'settings' | 'promptSettings' | 'licenses';

function App() {
  const [selectedFiles, setSelectedFiles] = useState<VideoFile[]>([]);
  const [settings, setSettings] = useState<AppSettings>({
    gemini_api_key: "",
    language: "japanese",
    temperature: 0,
    gemini_model: "gemini-2.5-pro",
    image_embed_frequency: "moderate"
  });
  const [isProcessing, setIsProcessing] = useState(false);
  const [generatedDocument, setGeneratedDocument] = useState("");
  const [currentPage, setCurrentPage] = useState<Page>('main');
  const [progressMessage, setProgressMessage] = useState("");
  const [progressStep, setProgressStep] = useState(0);
  const [totalSteps, setTotalSteps] = useState(0);
  const [showLogs, setShowLogs] = useState(false);
  const [saveDirectory, setSaveDirectory] = useState<string>("");
  const [currentPrompt, setCurrentPrompt] = useState<string>("");
  const [promptPresets, setPromptPresets] = useState<PromptPreset[]>([]);
  const [selectedPresetId, setSelectedPresetId] = useState<string>("");
  const [editingPreset, setEditingPreset] = useState<PromptPreset | null>(null);
  const [showEditModal, setShowEditModal] = useState(false);
  const [newPresetName, setNewPresetName] = useState("");
  const [newPresetPrompt, setNewPresetPrompt] = useState("");
  const [isDeleting, setIsDeleting] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [deleteTargetId, setDeleteTargetId] = useState<string | null>(null);

  const { logs, addLog, clearLogs } = useLogger();

  const handleSelectSaveDirectory = async () => {
    try {
      const directory = await invoke<string | null>("select_save_directory");
      if (directory) {
        setSaveDirectory(directory);
        addLog(`[SUCCESS] 保存先ディレクトリを選択: ${directory}`);
      }
    } catch (error) {
      addLog(`[ERROR] 保存先ディレクトリ選択エラー: ${error}`);
      console.error("Error selecting save directory:", error);
    }
  };

  useEffect(() => {
    loadSettings();
    loadPromptPresets();
    
    addLog("[INFO] Setting up progress update listener...");
    const unsubscribe = listen<ProgressUpdate>("progress_update", (event) => {
      const { message, step, total_steps } = event.payload;
      addLog(`[PROGRESS] Received progress update: ${step}/${total_steps} - ${message}`);
      setProgressMessage(message);
      setProgressStep(step);
      setTotalSteps(total_steps);
    });

    return () => {
      unsubscribe.then(f => f());
    };
  }, []);

  const handleFileSelect = async () => {
    addLog("[INFO] Starting file selection...");
    try {
      const files = await invoke<VideoFile[]>("select_video_files");
      addLog(`[SUCCESS] Selected ${files.length} files: ${files.map(f => f.name).join(", ")}`);
      setSelectedFiles(files);
      
      // 動画ファイルが選択された場合、最初のファイルのディレクトリを保存先として設定
      if (files.length > 0 && files[0].path) {
        const firstFilePath = files[0].path;
        const directoryPath = getDirectoryFromPath(firstFilePath);
        setSaveDirectory(directoryPath);
        addLog(`[INFO] 保存先を動画ディレクトリに設定: ${directoryPath}`);
      }
    } catch (error) {
      addLog(`[ERROR] Error selecting files: ${error}`);
      console.error("Error selecting files:", error);
    }
  };

  const handleRemoveFile = (index: number) => {
    const newFiles = selectedFiles.filter((_, i) => i !== index);
    setSelectedFiles(newFiles);
    
    // すべてのファイルが削除された場合、保存先ディレクトリもクリア
    if (newFiles.length === 0) {
      setSaveDirectory("");
      addLog("[INFO] すべてのファイルが削除されたため、保存先ディレクトリをクリアしました");
    } else if (newFiles.length > 0 && newFiles[0].path) {
      // 残りのファイルの最初のファイルのディレクトリを保存先として設定
      const firstFilePath = newFiles[0].path;
      const directoryPath = getDirectoryFromPath(firstFilePath);
      setSaveDirectory(directoryPath);
      addLog(`[INFO] 保存先を更新: ${directoryPath}`);
    }
  };

  const handleGenerateDocument = async () => {
    addLog("[START] Starting document generation process");
    
    if (selectedFiles.length === 0) {
      addLog("[ERROR] No video files selected");
      return;
    }
    
    if (!settings.gemini_api_key) {
      addLog("[ERROR] Gemini API key is not set");
      return;
    }

    let currentSaveDirectory = saveDirectory;
    if (!currentSaveDirectory) {
      addLog("[INFO] 保存先ディレクトリを選択してください");
      try {
        const directory = await invoke<string | null>("select_save_directory");
        if (directory) {
          currentSaveDirectory = directory;
          setSaveDirectory(directory);
          addLog(`[SUCCESS] 保存先ディレクトリを選択: ${directory}`);
        } else {
          addLog("[ERROR] 保存先ディレクトリが選択されていないため処理を中止します");
          return;
        }
      } catch (error) {
        addLog(`[ERROR] 保存先ディレクトリ選択エラー: ${error}`);
        return;
      }
    }

    const filename = generateFilename(selectedFiles);
    addLog(`[INFO] 生成予定ファイル名: ${filename}`);
    addLog(`[INFO] 保存先: ${currentSaveDirectory}`);

    setIsProcessing(true);
    setProgressMessage("処理を開始しています...");
    setProgressStep(0);
    setTotalSteps(0);
    setShowLogs(true);
    
    try {
      const result = await invoke<string>("generate_document", {
        files: selectedFiles,
        settings: {
          ...settings,
          custom_prompt: currentPrompt || undefined
        },
        saveDirectory: currentSaveDirectory
      });
      addLog("[SUCCESS] Document generation completed successfully");
      setGeneratedDocument(result);
      setProgressMessage("処理が完了しました！");

      try {
        const savedPath = await invoke<string>("save_document_to_file", {
          content: result,
          savePath: currentSaveDirectory,
          filename: filename
        });
        addLog(`[SUCCESS] ドキュメントを自動保存しました: ${savedPath}`);
      } catch (saveError) {
        addLog(`[ERROR] 自動保存に失敗しました: ${saveError}`);
      }
    } catch (error) {
      addLog(`[ERROR] Error generating document: ${error}`);
      setProgressMessage("エラーが発生しました。");
      console.error("Error generating document:", error);
    } finally {
      setIsProcessing(false);
      addLog("[COMPLETE] Document generation process finished");
    }
  };

  const handleGenerateFromYoutube = async (youtubeVideo: YouTubeVideoInfo) => {
    addLog(`[START] Starting YouTube document generation for: ${youtubeVideo.url}`);
    
    if (!settings.gemini_api_key) {
      addLog("[ERROR] Gemini API key is not set");
      return;
    }

    let currentSaveDirectory = saveDirectory;
    if (!currentSaveDirectory) {
      addLog("[INFO] 保存先ディレクトリを選択してください");
      try {
        const directory = await invoke<string | null>("select_save_directory");
        if (directory) {
          currentSaveDirectory = directory;
          setSaveDirectory(directory);
          addLog(`[SUCCESS] 保存先ディレクトリを選択: ${directory}`);
        } else {
          addLog("[ERROR] 保存先ディレクトリが選択されていないため処理を中止します");
          return;
        }
      } catch (error) {
        addLog(`[ERROR] 保存先ディレクトリ選択エラー: ${error}`);
        return;
      }
    }

    const filename = `${youtubeVideo.title.replace(/\s/g, '_')}.md`;
    addLog(`[INFO] 生成予定ファイル名: ${filename}`);
    addLog(`[INFO] 保存先: ${currentSaveDirectory}`);

    setIsProcessing(true);
    setProgressMessage("YouTube動画の処理を開始しています...");
    setProgressStep(0);
    setTotalSteps(1);
    setShowLogs(true);
    
    try {
      const result = await invoke<string>("generate_document_from_youtube", {
        youtubeVideo,
        settings: {
          ...settings,
          custom_prompt: currentPrompt || undefined,
        },
        saveDirectory: currentSaveDirectory,
        customPrompt: currentPrompt || null,
      });
      
      setGeneratedDocument(result);
      setProgressMessage("YouTube動画からのドキュメント生成が完了しました！");
      setProgressStep(1);
      addLog(`[SUCCESS] Document generated from YouTube video: ${youtubeVideo.title}`);
    } catch (error) {
      addLog(`[ERROR] Error generating document from YouTube: ${error}`);
      setProgressMessage("エラーが発生しました。");
      console.error("Error generating document from YouTube:", error);
    } finally {
      setIsProcessing(false);
      addLog("[COMPLETE] YouTube document generation process finished");
    }
  };

  const handleSaveSettings = async () => {
    try {
      await invoke("save_settings", { settings });
      addLog("[SUCCESS] Settings saved successfully");
      setCurrentPage('main');
    } catch (error) {
      addLog(`[ERROR] Error saving settings: ${error}`);
      console.error("Error saving settings:", error);
    }
  };

  const loadSettings = async () => {
    try {
      const savedSettings = await invoke<AppSettings | null>("load_settings");
      if (savedSettings) {
        // Ensure gemini_model has a default value if not set
        if (!savedSettings.gemini_model) {
          savedSettings.gemini_model = "gemini-2.5-pro";
        }
        setSettings(savedSettings);
        addLog(`[SUCCESS] Settings loaded successfully`);
      }
    } catch (error) {
      addLog(`[ERROR] Error loading settings: ${error}`);
      console.error("Error loading settings:", error);
    }
  };

  const loadPromptPresets = async () => {
    try {
      const presets = await invoke<PromptPreset[]>("load_prompt_presets");
      setPromptPresets(presets);
      addLog(`[SUCCESS] Loaded ${presets.length} prompt presets`);
    } catch (error) {
      addLog(`[ERROR] Error loading prompt presets: ${error}`);
      console.error("Error loading prompt presets:", error);
    }
  };

  const handlePromptPresetSelect = (presetId: string) => {
    setSelectedPresetId(presetId);
    if (presetId === "") {
      // 空の選択肢が選ばれた場合はプロンプトをクリア
      setCurrentPrompt("");
      return;
    }
    
    const preset = promptPresets.find(p => p.id === presetId);
    if (preset) {
      setCurrentPrompt(preset.prompt);
      addLog(`[SUCCESS] プロンプトプリセットを選択: ${preset.name}`);
    }
  };

  const handlePresetEdit = (preset: PromptPreset) => {
    if (preset.is_default) {
      alert('デフォルトプリセットは編集できません。');
      return;
    }
    setEditingPreset(preset);
    setNewPresetName(preset.name);
    setNewPresetPrompt(preset.prompt);
    setShowEditModal(true);
  };

  const handlePresetDeleteRequest = (presetId: string) => {
    if (isDeleting || showDeleteConfirm) return;
    
    const preset = promptPresets.find(p => p.id === presetId);
    if (preset?.is_default) {
      alert('デフォルトプリセットは削除できません。');
      return;
    }
    
    setDeleteTargetId(presetId);
    setShowDeleteConfirm(true);
  };

  const handleConfirmDelete = async () => {
    if (!deleteTargetId) return;

    setShowDeleteConfirm(false);
    setIsDeleting(true);
    
    try {
      const updatedPresets = promptPresets.filter(p => p.id !== deleteTargetId);
      await invoke("save_prompt_presets", { presets: updatedPresets });
      setPromptPresets(updatedPresets);
      addLog(`[SUCCESS] プリセットを削除しました`);
    } catch (error) {
      addLog(`[ERROR] プリセット削除エラー: ${error}`);
      console.error("Error deleting preset:", error);
    } finally {
      setIsDeleting(false);
      setDeleteTargetId(null);
    }
  };

  const handleCancelDelete = () => {
    setShowDeleteConfirm(false);
    setDeleteTargetId(null);
  };

  const handleNewPreset = () => {
    setEditingPreset(null);
    setNewPresetName("");
    setNewPresetPrompt("");
    setShowEditModal(true);
  };

  const handleSavePreset = async () => {
    if (!newPresetName.trim() || !newPresetPrompt.trim()) {
      alert("プリセット名とプロンプトの両方を入力してください。");
      return;
    }

    try {
      let updatedPresets;
      
      if (editingPreset) {
        updatedPresets = promptPresets.map(p => 
          p.id === editingPreset.id 
            ? { ...p, name: newPresetName, prompt: newPresetPrompt }
            : p
        );
      } else {
        const newPreset: PromptPreset = {
          id: `preset_${Date.now()}`,
          name: newPresetName,
          prompt: newPresetPrompt,
          is_default: false
        };
        updatedPresets = [...promptPresets, newPreset];
      }

      await invoke("save_prompt_presets", { presets: updatedPresets });
      setPromptPresets(updatedPresets);
      setShowEditModal(false);
      setEditingPreset(null);
      setNewPresetName("");
      setNewPresetPrompt("");
      addLog(`[SUCCESS] プリセットを保存しました: ${newPresetName}`);
    } catch (error) {
      addLog(`[ERROR] プリセット保存エラー: ${error}`);
      console.error("Error saving preset:", error);
    }
  };

  const handleImportXML = async () => {
    try {
      const importedPresets = await invoke<PromptPreset[]>("import_prompt_presets_from_file");
      setPromptPresets(importedPresets);
      addLog(`[SUCCESS] XMLファイルから${importedPresets.length}個のプリセットを読み込みました`);
    } catch (error) {
      addLog(`[ERROR] XMLファイル読み込みエラー: ${error}`);
      console.error("Error importing XML:", error);
    }
  };

  const handleExportXML = async () => {
    try {
      // デフォルトプリセットを除外してエクスポート
      const userPresets = promptPresets.filter(preset => !preset.is_default);
      await invoke("export_prompt_presets_to_file", { presets: userPresets });
      addLog(`[SUCCESS] ${userPresets.length}個のユーザープリセットをXMLファイルに出力しました`);
    } catch (error) {
      addLog(`[ERROR] XMLファイル出力エラー: ${error}`);
      console.error("Error exporting XML:", error);
    }
  };

  const handleUpdateSettingsWithSave = async (newSettings: AppSettings) => {
    setSettings(newSettings);
    try {
      await invoke("save_settings", { settings: newSettings });
      addLog(`[SUCCESS] 設定を更新しました`);
    } catch (error) {
      addLog(`[ERROR] 設定保存エラー: ${error}`);
    }
  };

  const renderPage = () => {
    switch (currentPage) {
      case 'settings':
        return (
          <Settings
            settings={settings}
            onUpdateSettings={setSettings}
            onClose={() => setCurrentPage('main')}
            onSave={handleSaveSettings}
            onNavigate={setCurrentPage}
          />
        );
      case 'promptSettings':
        return (
          <>
            <PromptSettings
              promptPresets={promptPresets}
              onClose={() => setCurrentPage('main')}
              onEditPreset={handlePresetEdit}
              onDeletePreset={handlePresetDeleteRequest}
              onNewPreset={handleNewPreset}
              onImportXML={handleImportXML}
              onExportXML={handleExportXML}
              isDeleting={isDeleting}
              showDeleteConfirm={showDeleteConfirm}
              deleteTargetId={deleteTargetId}
              onConfirmDelete={handleConfirmDelete}
              onCancelDelete={handleCancelDelete}
            />
            <PresetEditModal
              isOpen={showEditModal}
              editingPreset={editingPreset}
              presetName={newPresetName}
              presetPrompt={newPresetPrompt}
              onNameChange={setNewPresetName}
              onPromptChange={setNewPresetPrompt}
              onSave={handleSavePreset}
              onClose={() => setShowEditModal(false)}
            />
          </>
        );
      case 'licenses':
        return <LicenseDisplay onBack={() => setCurrentPage('settings')} />;
      case 'main':
      default:
        return (
          <MainDashboard
            settings={settings}
            onUpdateSettings={handleUpdateSettingsWithSave}
            selectedFiles={selectedFiles}
            onFileSelect={handleFileSelect}
            onRemoveFile={handleRemoveFile}
            currentPrompt={currentPrompt}
            onPromptChange={(prompt) => {
              setCurrentPrompt(prompt);
              if (selectedPresetId) {
                const selectedPreset = promptPresets.find(p => p.id === selectedPresetId);
                if (selectedPreset && selectedPreset.prompt !== prompt) {
                  setSelectedPresetId("");
                }
              }
            }}
            promptPresets={promptPresets}
            selectedPresetId={selectedPresetId}
            onPromptPresetSelect={handlePromptPresetSelect}
            saveDirectory={saveDirectory}
            onSelectSaveDirectory={handleSelectSaveDirectory}
            onGenerateDocument={handleGenerateDocument}
            onGenerateFromYoutube={handleGenerateFromYoutube}
            isProcessing={isProcessing}
            progressMessage={progressMessage}
            progressStep={progressStep}
            totalSteps={totalSteps}
            logs={logs}
            showLogs={showLogs}
            onToggleLogs={() => setShowLogs(!showLogs)}
            onClearLogs={clearLogs}
            generatedDocument={generatedDocument}
            onShowSettings={() => setCurrentPage('settings')}
            onShowPromptSettings={() => setCurrentPage('promptSettings')}
            generateFilename={generateFilename}
          />
        );
    }
  };

  return <div className="App">{renderPage()}</div>;
}

export default App;
