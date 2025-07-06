# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Tauri application that combines a React frontend with a Rust backend. The project is called "document_encoder" and appears to be a desktop application for document encoding functionality.

## What We're Building

**For detailed project structure and requirements, refer to [@doc/spec.md](doc/spec.md) - this contains the complete specification including:**

- **Core Purpose**: Desktop application that automatically generates text-based documents (manuals or specifications) from video files using Google's Gemini Pro API
- **Supported Platforms**: macOS and Windows
- **Video Processing**: Supports multiple formats (mp4, mov, avi, etc.) with automatic splitting for videos over 1 hour
- **Document Generation**: Two modes - Manual creation mode and Specification creation mode
- **Multi-language Support**: Japanese and English output
- **Advanced Features**: Custom prompt presets, progress tracking, XML import/export of presets

**Backend Module Structure** (Rust):
- `lib.rs` (391 lines) - Main library with Tauri commands and initialization
- `types.rs` (71 lines) - Type definitions for VideoFile, AppSettings, etc.
- `file.rs` (90 lines) - File operation module with dialogs
- `gemini.rs` (509 lines) - Gemini API integration with progress tracking
- `video.rs` (213 lines) - Video processing with ffmpeg/ffprobe integration

**Frontend Module Structure** (React/TypeScript):
- `App.tsx` - Main application component with screen routing
- `types.ts` - Type definitions matching backend structures
- `utils/fileUtils.ts` - File manipulation utilities
- `hooks/useLogger.ts` - Custom logging hook
- `components/` - Screen components: ApiSettings, PromptSettings, PresetEditModal, MainDashboard

## Architecture

**Frontend (React + TypeScript)**
- Built with Vite for fast development and bundling
- Uses React 18 with TypeScript for type safety
- Main application entry point: `src/main.tsx`
- Primary component: `src/App.tsx`

**Backend (Rust)**
- Tauri framework for desktop application wrapper
- Main library code in `src-tauri/src/lib.rs`
- Tauri commands for frontend-backend communication
- Currently implements a basic "greet" command as an example

**Build System**
- Frontend: Vite bundler with React plugin
- Backend: Cargo for Rust compilation
- Tauri CLI orchestrates the build process

## Development Testing

**After making any development changes, ALWAYS run the following command to verify the application works correctly:**

```bash
npm run tauri dev
```

This command will:
- Start the Tauri development environment
- Launch both the React frontend and Rust backend
- Open the desktop application window
- Allow you to test all functionality including video file selection, Gemini API integration, and document generation

**Testing checklist when running `npm run tauri dev`:**
- Application launches without errors
- All UI components render correctly
- File selection dialogs work properly
- API settings can be configured
- Prompt settings and presets function as expected
- Video processing and document generation complete successfully
- Progress tracking displays correctly
- Generated documents can be saved properly

## Development Commands

**Frontend Development**
- `npm run dev` - Start Vite development server
- `npm run build` - Build frontend for production (runs TypeScript compiler then Vite build)
- `npm run preview` - Preview production build

**Full Application Development**
- `npm run tauri dev` - Start Tauri development mode (runs both frontend and backend)
- `npm run tauri build` - Build complete desktop application

**Note**: The `tauri.conf.json` references `deno task dev` and `deno task build` commands, but the project actually uses npm scripts. This may be a configuration inconsistency.

## Key Configuration Files

- `package.json` - Node.js dependencies and scripts
- `src-tauri/Cargo.toml` - Rust dependencies and build configuration
- `src-tauri/tauri.conf.json` - Tauri application configuration
- `tsconfig.json` - TypeScript compiler configuration
- `vite.config.ts` - Vite bundler configuration

## Frontend-Backend Communication

The application uses Tauri's `invoke` system to call Rust functions from the frontend. Example:
- Frontend calls `invoke("greet", { name })` in `src/App.tsx:12`
- Backend handles this in the `greet` function in `src-tauri/src/lib.rs:3`

## Development Notes

- The application runs on `localhost:1420` during development
- TypeScript is configured with strict mode enabled
- The Rust backend uses the `tauri_plugin_opener` for opening external links
- Application identifier is set to `jp.ynr.docenc`
