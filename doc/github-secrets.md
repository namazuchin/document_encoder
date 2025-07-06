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

## トラブルシューティング

- 署名エラーが発生する場合は、証明書の有効期限とTeam IDを確認
- notarization（公証）に失敗する場合は、Apple IDとApp固有パスワードを確認
- ビルドが失敗する場合は、GitHub Actions のログを詳細に確認
