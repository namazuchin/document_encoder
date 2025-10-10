export const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
};

export const formatDuration = (seconds: number): string => {
  if (seconds < 0) return "不明";
  
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainingSeconds = Math.floor(seconds % 60);
  
  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${remainingSeconds.toString().padStart(2, '0')}`;
  } else {
    return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`;
  }
};

// クロスプラットフォームで安全なファイル名へ変換
export const sanitizeFilename = (name: string): string => {
  if (!name) return "untitled";

  // 無効文字をアンダースコアへ置換（Windows/Unix共通の危険文字 + 制御文字）
  let safe = name
    .replace(/[<>:"/\\|?*\u0000-\u001F]/g, "_")
    .replace(/\u007F/g, "_");

  // 連続空白をアンダースコアに、連続アンダースコアを1つに
  safe = safe.replace(/\s+/g, "_").replace(/_+/g, "_");

  // 末尾のドット/スペースを除去
  safe = safe.replace(/[.\s]+$/g, "");

  // Windows予約名を回避（拡張子なしのベースとして扱う）
  const lower = safe.toLowerCase();
  const reserved = new Set([
    "con","prn","aux","nul",
    "com1","com2","com3","com4","com5","com6","com7","com8","com9",
    "lpt1","lpt2","lpt3","lpt4","lpt5","lpt6","lpt7","lpt8","lpt9"
  ]);
  if (reserved.has(lower)) {
    safe = `${safe}_`;
  }

  // 長さ制限（過度なパス長を避けるため適度に制限）
  if (safe.length > 120) {
    safe = safe.slice(0, 120);
  }

  // 空になった場合のフォールバック
  if (!safe) return "untitled";

  return safe;
};

export const generateFilename = (files: { name: string }[]): string => {
  if (files.length === 0) return "document.md";
  
  const firstFile = files[0];
  const filename = firstFile.name;
  const nameWithoutExt = filename.replace(/\.[^/.]+$/, "");
  const safeBase = sanitizeFilename(nameWithoutExt);
  return `${safeBase || "document"}.md`;
};

export const getDirectoryFromPath = (filePath: string): string => {
  // Handle both Windows (\) and Unix (/) path separators
  const lastBackslash = filePath.lastIndexOf('\\');
  const lastSlash = filePath.lastIndexOf('/');
  const lastSeparator = Math.max(lastBackslash, lastSlash);
  
  if (lastSeparator === -1) {
    // No separator found, return empty string or current directory
    return "";
  }
  
  return filePath.substring(0, lastSeparator);
};
