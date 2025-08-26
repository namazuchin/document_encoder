export interface VideoFile {
  path: string;
  name: string;
  size: number;
}

export type VideoQuality = "NoConversion" | "1080p" | "720p" | "480p";

export type ImageEmbedFrequency = "minimal" | "moderate" | "detailed";

export type FrameExtractionMethod = "standard" | "fast" | "multiple";

export interface AppSettings {
  gemini_api_key: string;
  language: string;
  temperature: number;
  custom_prompt?: string;
  gemini_model?: string;
  embed_images?: boolean;
  image_embed_frequency?: ImageEmbedFrequency;
  video_quality?: VideoQuality;
  hardware_encoding?: boolean;
  // 実験用機能
  enable_experimental_features?: boolean;
  frame_extraction_method?: FrameExtractionMethod;
}

export interface PromptPreset {
  id: string;
  name: string;
  prompt: string;
  is_default?: boolean;
}

export interface ProgressUpdate {
  message: string;
  step: number;
  total_steps: number;
}

export interface YouTubeVideoInfo {
  url: string;
  title: string;
}

export type VideoSource = 
  | { type: 'local'; files: VideoFile[] }
  | { type: 'youtube'; video: YouTubeVideoInfo };
