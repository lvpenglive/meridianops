#!/usr/bin/env bash
# MeridianOps 一键启动（无需单独 Nginx）
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

CFG="${GATEWAY_CONFIG:-$ROOT/config/gateway-config.toml}"
PID_FILE="$ROOT/run/gateway.pid"
LOG_FILE="$ROOT/run/gateway.log"
BIN="$ROOT/bin/meridianops-gateway"

mkdir -p "$ROOT/run" "$ROOT/config"

if [[ ! -x "$BIN" ]]; then
  echo "缺少可执行文件: $BIN" >&2
  exit 1
fi

if [[ ! -f "$CFG" ]]; then
  if [[ -f "$ROOT/config/gateway-config.deploy.toml.example" ]]; then
    cp "$ROOT/config/gateway-config.deploy.toml.example" "$CFG"
    echo "已生成 $CFG ，请先编辑数据库连接等信息后重新执行 ./start.sh"
    exit 1
  fi
  echo "缺少配置: $CFG" >&2
  exit 1
fi

# 确保托管本包内的 portal/
if ! grep -qE '^\s*portal_dir\s*=' "$CFG"; then
  echo "警告: 配置里没有 portal_dir，将通过环境变量指定" >&2
fi
export MERIDIANOPS_PORTAL_DIR="${MERIDIANOPS_PORTAL_DIR:-$ROOT/portal}"

if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
  echo "已在运行 pid=$(cat "$PID_FILE")，先 ./stop.sh 或忽略"
  exit 0
fi

nohup "$BIN" --config "$CFG" >>"$LOG_FILE" 2>&1 &
echo $! >"$PID_FILE"
sleep 1
if kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
  BIND=$(grep -E '^\s*bind\s*=' "$CFG" | head -1 | sed 's/.*=\s*"\?\([^"]*\)"\?.*/\1/' || true)
  echo "Gateway 已启动 pid=$(cat "$PID_FILE") bind=${BIND:-见配置}"
  echo "浏览器访问: http://127.0.0.1:8800/  （若 bind 改过请用对应地址）"
  echo "日志: $LOG_FILE"
else
  echo "启动失败，查看日志: $LOG_FILE" >&2
  tail -n 40 "$LOG_FILE" >&2 || true
  rm -f "$PID_FILE"
  exit 1
fi
