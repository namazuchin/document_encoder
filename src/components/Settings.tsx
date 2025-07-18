import { AppSettings } from '../types';
import { FaSave, FaTimes, FaKey, FaThermometerHalf, FaRobot, FaMicrochip } from 'react-icons/fa';

interface SettingsProps {
  settings: AppSettings;
  onUpdateSettings: (settings: AppSettings) => void;
  onClose: () => void;
  onSave: () => void;
}

export default function Settings({ settings, onUpdateSettings, onClose, onSave }: SettingsProps) {
  return (
    <div className="api-settings-container">
      <div className="api-settings-content">
        <h1>設定</h1>
        <div className="settings-form">
          <div className="form-group">
            <label htmlFor="apiKey"><FaKey className="icon" /> Gemini API Key:</label>
            <input
              type="password"
              id="apiKey"
              value={settings.gemini_api_key}
              onChange={(e) => onUpdateSettings({ ...settings, gemini_api_key: e.target.value })}
              placeholder="API keyを入力してください"
            />
          </div>
          
          <div className="form-group">
            <label htmlFor="temperature"><FaThermometerHalf className="icon" /> Temperature (0.0 - 1.0):</label>
            <input
              type="number"
              id="temperature"
              min="0"
              max="1"
              step="0.1"
              value={settings.temperature}
              onChange={(e) => onUpdateSettings({ ...settings, temperature: parseFloat(e.target.value) || 0 })}
              placeholder="0.0"
            />
            <small style={{ color: '#666', fontSize: '12px', marginTop: '4px', display: 'block' }}>
              創造性の制御: 0=確定的、1=創造的
            </small>
          </div>
          
          <div className="form-group">
            <label htmlFor="geminiModel"><FaRobot className="icon" /> Gemini Model:</label>
            <select
              id="geminiModel"
              value={settings.gemini_model || 'gemini-2.5-pro'}
              onChange={(e) => onUpdateSettings({ ...settings, gemini_model: e.target.value })}
            >
              <option value="gemini-2.5-pro">gemini-2.5-pro</option>
              <option value="gemini-2.5-flash">gemini-2.5-flash</option>
              <option value="gemini-2.5-pro-preview-06-05">gemini-2.5-pro-preview-06-05</option>
            </select>
            <small style={{ color: '#666', fontSize: '12px', marginTop: '4px', display: 'block' }}>
              使用するGeminiモデルを選択してください、flashモデルでの生成精度は使い物にならない程度に低いので注意してください。
            </small>
          </div>
          
          <div className="form-group">
            <label htmlFor="hardwareEncoding">
              <FaMicrochip className="icon" /> 
              <input
                type="checkbox"
                id="hardwareEncoding"
                checked={settings.hardware_encoding || false}
                onChange={(e) => onUpdateSettings({ ...settings, hardware_encoding: e.target.checked })}
                style={{ marginLeft: '8px', marginRight: '8px' }}
              />
              ハードウェアエンコードを有効にする
            </label>
            <small style={{ color: '#666', fontSize: '12px', marginTop: '4px', display: 'block' }}>
              利用可能な場合、ハードウェアエンコードを使用して動画処理を高速化します。
            </small>
          </div>
          
          <div className="button-group">
            <button onClick={onSave}>
              <FaSave className="icon" /> 保存
            </button>
            <button onClick={onClose}>
              <FaTimes className="icon" /> キャンセル
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
