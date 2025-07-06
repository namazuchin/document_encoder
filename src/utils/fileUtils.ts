export const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
};

export const generateFilename = (files: { name: string }[]): string => {
  if (files.length === 0) return "document.md";
  
  const firstFile = files[0];
  const filename = firstFile.name;
  const nameWithoutExt = filename.replace(/\.[^/.]+$/, "");
  return `${nameWithoutExt}.md`;
};