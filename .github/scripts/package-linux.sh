#!/usr/bin/env bash
# 把网关 + 门户 + 一键脚本打成 tar.gz（Gateway 可直接托管门户，无需 Nginx）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

: "${PACKAGE_NAME:?PACKAGE_NAME is required}"
: "${PACKAGE_LABEL:?PACKAGE_LABEL is required}"

BIN="gateway/target/release/meridianops-gateway"
PORTAL_DIST="portal/dist"
CFG_EXAMPLE="gateway/gateway-config.toml.example"
DEPLOY_CFG="deploy/gateway-config.deploy.toml.example"

if [[ ! -f "$BIN" ]]; then
  echo "missing binary: $BIN" >&2
  exit 1
fi
if [[ ! -d "$PORTAL_DIST" ]]; then
  echo "missing portal build: $PORTAL_DIST" >&2
  exit 1
fi

STAGE="dist/${PACKAGE_NAME}"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/portal" "$STAGE/config" "$STAGE/run"

cp "$BIN" "$STAGE/bin/"
chmod +x "$STAGE/bin/meridianops-gateway"
if command -v strip >/dev/null 2>&1; then
  strip "$STAGE/bin/meridianops-gateway" || true
fi
cp -a "$PORTAL_DIST"/. "$STAGE/portal/"
cp "$CFG_EXAMPLE" "$STAGE/config/gateway-config.toml.example"
cp "$DEPLOY_CFG" "$STAGE/config/gateway-config.deploy.toml.example"
# 部署示例里 portal_dir 用相对包根目录
sed 's#portal_dir = "portal"#portal_dir = "portal"#' \
  "$DEPLOY_CFG" > "$STAGE/config/gateway-config.deploy.toml.example"

cp deploy/start.sh deploy/stop.sh deploy/status.sh "$STAGE/"
chmod +x "$STAGE/start.sh" "$STAGE/stop.sh" "$STAGE/status.sh"

{
  echo "package=${PACKAGE_NAME}"
  echo "label=${PACKAGE_LABEL}"
  echo "git=$(git rev-parse HEAD 2>/dev/null || echo unknown)"
  echo "ref=${GITHUB_REF:-}"
  echo "built_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "$STAGE/BUILD.txt"

cat > "$STAGE/README-DEPLOY.txt" <<'EOF'
MeridianOps 一键部署包

内容
  bin/meridianops-gateway
  portal/                         前端静态资源（由 Gateway 托管）
  config/gateway-config.deploy.toml.example
  start.sh / stop.sh / status.sh

三步启动
  1. tar xzf 本包 && cd meridianops-*
  2. cp config/gateway-config.deploy.toml.example config/gateway-config.toml
     只改 [database].url（以及 jwt_secret）
  3. ./start.sh
  4. 浏览器打开 http://127.0.0.1:8800/

说明
  - 无需单独 Nginx：Gateway 同时提供 /api 与门户页面
  - 默认 bind 127.0.0.1:8800，满足本机安全校验
  - 可选 zabbix-ctl：另解压 ctl 包，service_token 与 [zabbix_ctl].service_token 填成一样
  - 停止: ./stop.sh
EOF

mkdir -p dist
tar -C dist -czf "dist/${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"
ls -lh "dist/${PACKAGE_NAME}.tar.gz"
echo "staged ${PACKAGE_NAME} (${PACKAGE_LABEL})"
