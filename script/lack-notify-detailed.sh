#!/bin/bash

# Slack詳細通知スクリプト
# Claude Codeの操作内容を詳細にSlackへ通知します

# 設定ファイルの読み込み
SCRIPT_DIR=$(dirname "$0")
CONFIG_FILE="${SCRIPT_DIR}/.env"

# 設定ファイルが存在する場合は読み込む
if [ -f "$CONFIG_FILE" ]; then
    source "$CONFIG_FILE"
fi

# Slack Webhook URLの確認
if [ -z "$SLACK_WEBHOOK_URL" ]; then
    echo "エラー: SLACK_WEBHOOK_URLが設定されていません" >&2
    echo "scripts/.env ファイルに SLACK_WEBHOOK_URL を設定してください" >&2
    exit 1
fi

# タイムスタンプ
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# JSONデータを読み込む
JSON_INPUT=$(cat)

# 基本情報の抽出
SESSION_ID=$(echo "$JSON_INPUT" | jq -r '.session_id // "N/A"')
TOOL_NAME=$(echo "$JSON_INPUT" | jq -r '.tool_name // empty')
EVENT_TYPE="${1:-PostToolUse}"  # デフォルトはPostToolUse

# ツールに応じたメッセージとアイコンの生成
case "$TOOL_NAME" in
    "Write")
        ICON=":pencil2:"
        TITLE="ファイルを作成しました"
        FILE_PATH=$(echo "$JSON_INPUT" | jq -r '.tool_input.file_path // .tool_input.path // "不明"')
        DETAILS="ファイル: \`${FILE_PATH}\`"
        ;;
    "Edit"|"MultiEdit")
        ICON=":memo:"
        TITLE="ファイルを編集しました"
        FILE_PATH=$(echo "$JSON_INPUT" | jq -r '.tool_input.file_path // .tool_input.path // "不明"')
        if [ "$TOOL_NAME" = "MultiEdit" ]; then
            EDIT_COUNT=$(echo "$JSON_INPUT" | jq -r '.tool_input.edits | length // 0')
            DETAILS="ファイル: \`${FILE_PATH}\`\n編集箇所: ${EDIT_COUNT}件"
        else
            DETAILS="ファイル: \`${FILE_PATH}\`"
        fi
        ;;
    "Bash")
        ICON=":zap:"
        TITLE="コマンドを実行しました"
        COMMAND=$(echo "$JSON_INPUT" | jq -r '.tool_input.command // "不明"')
        SUCCESS=$(echo "$JSON_INPUT" | jq -r '.tool_response.success // true')
        STATUS=$([ "$SUCCESS" = "true" ] && echo "成功" || echo "失敗")
        # コマンドが長い場合は省略
        if [ ${#COMMAND} -gt 100 ]; then
            COMMAND_DISPLAY="${COMMAND:0:97}..."
        else
            COMMAND_DISPLAY="$COMMAND"
        fi
        DETAILS="コマンド: \`${COMMAND_DISPLAY}\`\n結果: ${STATUS}"
        ;;
    "Read"|"NotebookRead")
        ICON=":book:"
        TITLE="ファイルを読み取りました"
        FILE_PATH=$(echo "$JSON_INPUT" | jq -r '.tool_input.file_path // .tool_input.notebook_path // .tool_input.path // "不明"')
        DETAILS="ファイル: \`${FILE_PATH}\`"
        ;;
    "TodoWrite")
        ICON=":white_check_mark:"
        TITLE="TODOリストを更新しました"
        TODO_COUNT=$(echo "$JSON_INPUT" | jq -r '.tool_input.todos | length // 0')
        DETAILS="タスク数: ${TODO_COUNT}件"
        ;;
    "Grep"|"Glob")
        ICON=":mag:"
        TITLE="ファイル検索を実行しました"
        PATTERN=$(echo "$JSON_INPUT" | jq -r '.tool_input.pattern // "不明"')
        DETAILS="パターン: \`${PATTERN}\`"
        ;;
    "LS")
        ICON=":file_folder:"
        TITLE="ディレクトリを一覧表示しました"
        PATH=$(echo "$JSON_INPUT" | jq -r '.tool_input.path // "不明"')
        DETAILS="パス: \`${PATH}\`"
        ;;
    "WebFetch"|"WebSearch")
        ICON=":globe_with_meridians:"
        TITLE="Web情報を取得しました"
        if [ "$TOOL_NAME" = "WebFetch" ]; then
            URL=$(echo "$JSON_INPUT" | jq -r '.tool_input.url // "不明"')
            DETAILS="URL: ${URL}"
        else
            QUERY=$(echo "$JSON_INPUT" | jq -r '.tool_input.query // "不明"')
            DETAILS="検索: ${QUERY}"
        fi
        ;;
    "")
        # Stopイベントの場合
        if [ "$EVENT_TYPE" = "Stop" ]; then
            ICON=":checkered_flag:"
            TITLE="セッションが完了しました"
            DETAILS="セッションID: \`${SESSION_ID}\`"
        else
            ICON=":question:"
            TITLE="操作を実行しました"
            DETAILS="詳細情報なし"
        fi
        ;;
    *)
        ICON=":gear:"
        TITLE="${TOOL_NAME}を実行しました"
        DETAILS="ツール: ${TOOL_NAME}"
        ;;
esac

# Slack通知の送信
RESPONSE=$(curl -s -X POST "$SLACK_WEBHOOK_URL" \
    -H 'Content-Type: application/json' \
    -d @- <<EOF
{
    "username": "Claude Code",
    "icon_emoji": ":robot_face:",
    "blocks": [
        {
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": "${ICON} *${TITLE}*"
            }
        },
        {
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": "${DETAILS}"
            }
        },
        {
            "type": "context",
            "elements": [
                {
                    "type": "mrkdwn",
                    "text": "時刻: ${TIMESTAMP} | セッション: \`${SESSION_ID:0:8}...\`"
                }
            ]
        }
    ]
}
EOF
)

# レスポンスの確認
if [ "$RESPONSE" = "ok" ]; then
    echo "Slack通知を送信しました: ${TITLE}"
else
    echo "Slack通知の送信に失敗しました: $RESPONSE" >&2
    exit 1
fi
