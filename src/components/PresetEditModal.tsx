import { PromptPreset } from '../types';

interface PresetEditModalProps {
  isOpen: boolean;
  editingPreset: PromptPreset | null;
  presetName: string;
  presetPrompt: string;
  onNameChange: (name: string) => void;
  onPromptChange: (prompt: string) => void;
  onSave: () => void;
  onClose: () => void;
}

export default function PresetEditModal({
  isOpen,
  editingPreset,
  presetName,
  presetPrompt,
  onNameChange,
  onPromptChange,
  onSave,
  onClose
}: PresetEditModalProps) {
  if (!isOpen) return null;

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <h3>{editingPreset ? 'プリセット編集' : '新規プリセット作成'}</h3>
        <div className="form-group">
          <label htmlFor="presetName">プリセット名:</label>
          <input
            type="text"
            id="presetName"
            value={presetName}
            onChange={(e) => onNameChange(e.target.value)}
            placeholder="プリセット名を入力..."
          />
        </div>
        <div className="form-group">
          <label htmlFor="presetPrompt">プロンプト:</label>
          <textarea
            id="presetPrompt"
            value={presetPrompt}
            onChange={(e) => onPromptChange(e.target.value)}
            placeholder="プロンプトを入力..."
            rows={6}
          />
        </div>
        <div className="modal-buttons">
          <button onClick={onSave}>
            {editingPreset ? '更新' : '作成'}
          </button>
          <button onClick={onClose}>キャンセル</button>
        </div>
      </div>
    </div>
  );
}
