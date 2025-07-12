# GitHub Secretsの設定

macOSビルドと署名のためにGitHubリポジトリで設定する必要があるシークレット

## 必要なシークレット

### Apple Developer Account関連

1. **APPLE_CERTIFICATE**
   - Apple Developer証明書（.p12形式）をbase64エンコードした文字列
   - 取得方法：
     ```bash
     # .p12ファイルをbase64エンコード
     base64 -i path/to/certificate.p12 | pbcopy
     ```

2. **APPLE_CERTIFICATE_PASSWORD**
   - Apple Developer証明書のパスワード

3. **APPLE_SIGNING_IDENTITY**
   - 署名に使用するidentity名
   - 例：`"Developer ID Application: Your Name (TEAM_ID)"`

4. **APPLE_ID**
   - Apple Developer AccountのApple ID（メールアドレス）

5. **APPLE_PASSWORD**
   - App Store Connect用のApp固有パスワード
   - 取得方法：Apple ID設定 > サインインとセキュリティ > App用パスワード

6. **APPLE_TEAM_ID**
   - Apple Developer TeamのID
   - Apple Developer Consoleで確認可能

### Google Drive アップロード関連

7. **GOOGLE_DRIVE_CREDENTIALS**
   - Google Service Accountの認証情報（JSON形式）をbase64エンコードした文字列
   - 取得方法：
     ```bash
     # Service Accountのキーファイルをbase64エンコード
     base64 -i path/to/service-account-key.json | pbcopy
     ```

8. **GOOGLE_DRIVE_FOLDER_ID**
   - アップロード先のGoogle DriveフォルダID
   - Google DriveのURLから取得：`https://drive.google.com/drive/folders/[FOLDER_ID]`

## 設定手順

1. GitHubリポジトリの「Settings」タブに移動
2. 左メニューから「Secrets and variables」→「Actions」を選択
3. 「New repository secret」をクリック
4. 上記のシークレットを一つずつ追加

## 注意事項

- 証明書とパスワードは慎重に管理する
- 定期的に証明書の有効期限を確認する
- チームメンバーと共有する場合は、適切な権限管理を行う

## 証明書の作成方法

1. Apple Developer Consoleにログイン
2. 「Certificates, Identifiers & Profiles」に移動
3. 「Certificates」→「+」ボタンをクリック
4. 「Developer ID Application」を選択
5. CSR（Certificate Signing Request）をアップロード
6. 作成された証明書をダウンロード
7. キーチェーンアクセスで.p12形式でエクスポート

## Google Service Account設定手順

1. [Google Cloud Console](https://console.cloud.google.com/)にアクセス
2. プロジェクトを作成または選択
3. 「APIとサービス」→「認証情報」に移動
4. 「認証情報を作成」→「サービスアカウント」を選択
5. サービスアカウント名を入力して作成
6. 作成したサービスアカウントをクリック
7. 「キー」タブ→「キーを追加」→「新しいキーを作成」
8. JSON形式を選択してダウンロード
9. [Google Drive API](https://console.developers.google.com/apis/api/drive.googleapis.com)を有効化
10. Google Driveでアップロード先フォルダをサービスアカウントのメールアドレスと共有

## トラブルシューティング

### Apple署名関連
- 署名エラーが発生する場合は、証明書の有効期限とTeam IDを確認
- notarization（公証）に失敗する場合は、Apple IDとApp固有パスワードを確認
- ビルドが失敗する場合は、GitHub Actions のログを詳細に確認

### Google Drive アップロード関連
- アップロードエラーが発生する場合は、フォルダの共有設定を確認
- 認証エラーの場合は、Service Account JSONファイルのbase64エンコードを確認
- API制限エラーの場合は、Google Drive APIが有効化されているか確認
