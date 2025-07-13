import { PromptPreset } from '../types';
import { FaPlus, FaUpload, FaDownload, FaTimes, FaEdit, FaTrash, FaExclamationTriangle } from 'react-icons/fa';

interface PromptSettingsProps {
  promptPresets: PromptPreset[];
  onClose: () => void;
  onEditPreset: (preset: PromptPreset) => void;
  onDeletePreset: (presetId: string) => void;
  onNewPreset: () => void;
  onImportXML: () => void;
  onExportXML: () => void;
  isDeleting: boolean;
  showDeleteConfirm: boolean;
  deleteTargetId: string | null;
  onConfirmDelete: () => void;
  onCancelDelete: () => void;
}

export default function PromptSettings({
  promptPresets,
  onClose,
  onEditPreset,
  onDeletePreset,
  onNewPreset,
  onImportXML,
  onExportXML,
  isDeleting,
  showDeleteConfirm,
  deleteTargetId,
  onConfirmDelete,
  onCancelDelete
}: PromptSettingsProps) {
  return (
    <div className="prompt-settings-container">
      <div className="prompt-settings-content">
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
                    <span className="preset-preview">{preset.prompt.substring(0, 80)}...</span>
                  </div>
                  <div className="preset-actions">
                    {!preset.is_default && (
                      <>
                        <button onClick={(e) => { e.stopPropagation(); onEditPreset(preset); }}>
                          <FaEdit className="icon" /> 編集
                        </button>
                        <button 
                          onClick={(e) => { e.stopPropagation(); onDeletePreset(preset.id); }}
                          disabled={isDeleting || showDeleteConfirm}
                          className={isDeleting ? 'deleting' : ''}
                        >
                          <FaTrash className="icon" /> {isDeleting ? '削除中...' : '削除'}
                        </button>
                      </>
                    )}
                  </div>
                </div>
              ))}
            </div>
            <div className="button-group">
              <button onClick={onNewPreset}>
                <FaPlus className="icon" /> 新規プリセット作成
              </button>
              <button onClick={onImportXML}>
                <FaUpload className="icon" /> XMLファイルから読み込み
              </button>
              <button onClick={onExportXML}>
                <FaDownload className="icon" /> XMLファイルにエクスポート
              </button>
            </div>
          </div>
          
          <div className="button-group">
            <button onClick={onClose}>
              <FaTimes className="icon" /> 閉じる
            </button>
          </div>
        </div>

        {showDeleteConfirm && (
          <div className="modal-overlay">
            <div className="modal-content delete-confirm">
              <h3>プリセット削除の確認</h3>
              <p>
                「{promptPresets.find(p => p.id === deleteTargetId)?.name || 'このプリセット'}」を削除しますか？
              </p>
              <p className="warning-text">
                <FaExclamationTriangle className="warning-icon" /> この操作は取り消せません。
              </p>
              <div className="modal-buttons">
                <button className="delete-confirm-btn" onClick={onConfirmDelete}>
                  削除する
                </button>
                <button onClick={onCancelDelete}>キャンセル</button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
