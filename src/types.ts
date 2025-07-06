export interface VideoFile {
  path: string;
  name: string;
  size: number;
}

export interface AppSettings {
  gemini_api_key: string;
  language: string;
  temperature: number;
  custom_prompt?: string;
  gemini_model?: string;
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