# Document Encoder

動画ファイルからAI（Gemini Pro）を使ってドキュメント（マニュアル・仕様書）を自動生成するデスクトップアプリケーションです。

<img src="./doc/icon.png" width="256" />

## 概要

このアプリケーションは、動画ファイルの内容を解析し、テキストベースのドキュメントを自動で生成します。GoogleのGemini Pro APIを活用し、macOSおよびWindows上で動作します。

## 主な機能

- **動画からのドキュメント生成**: 複数の動画ファイル（.mp4, .movなど）を選択し、ドキュメントを生成します。
- **2つの生成モード**:
    - **マニュアルモード**: 動画の内容を手順や操作方法として解説します。
    - **仕様書モード**: 動画の内容を機能や動作の仕様として記述します。
- **多言語対応**: 生成するドキュメントの言語を日本語または英語から選択できます。
- **長時間動画のサポート**: 1時間を超える動画は自動的に分割して処理されます。
- **進捗表示**: ファイル処理、アップロード、ドキュメント生成の進捗がリアルタイムで表示されます。

## 技術スタック

- **フレームワーク**: [Tauri](https://tauri.app/)
- **フロントエンド**: [React](https://react.dev/), [TypeScript](https://www.typescriptlang.org/)
- **バックエンド**: [Rust](https://www.rust-lang.org/)
- **外部API**: [Google Gemini API](https://ai.google.dev/)
- **動画処理**: `ffmpeg`, `ffprobe`

## 開発環境のセットアップ

### 推奨IDE

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

### コマンド

```bash
# 依存関係のインストール
npm install

# 開発サーバーの起動
npm run tauri dev

# アプリケーションのビルド
npm run tauri build
```

## ライセンス

このプロジェクトは[MITライセンス](LICENSE)の下で公開されています。
