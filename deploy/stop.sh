#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
PID_FILE="$ROOT/run/gateway.pid"
if [[ ! -f "$PID_FILE" ]]; then
  echo "未发现 pid 文件，可能未启动"
  exit 0
fi
PID="$(cat "$PID_FILE")"
if kill -0 "$PID" 2>/dev/null; then
  kill "$PID" || true
  sleep 1
  kill -9 "$PID" 2>/dev/null || true
  echo "已停止 pid=$PID"
else
  echo "进程不存在 pid=$PID"
fi
rm -f "$PID_FILE"
