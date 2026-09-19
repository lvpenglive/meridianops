#!/usr/bin/env bash
# 把网关 release 二进制 + 门户静态资源打成 tar.gz
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

: "${PACKAGE_NAME:?PACKAGE_NAME is required}"
: "${PACKAGE_LABEL:?PACKAGE_LABEL is required}"

BIN="gateway/target/release/meridianops-gateway"
PORTAL_DIST="portal/dist"
CFG_EXAMPLE="gateway/gateway-config.toml.example"

if [[ ! -f "$BIN" ]]; then
  echo "missing binary: $BIN" >&2
  exit 1
fi
if [[ ! -d "$PORTAL_DIST" ]]; then
  echo "missing portal build: $PORTAL_DIST" >&2
  exit 1
fi
if [[ ! -f "$CFG_EXAMPLE" ]]; then
  echo "missing config example: $CFG_EXAMPLE" >&2
  exit 1
fi

STAGE="dist/${PACKAGE_NAME}"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/portal" "$STAGE/config"

cp "$BIN" "$STAGE/bin/"
chmod +x "$STAGE/bin/meridianops-gateway"
if command -v strip >/dev/null 2>&1; then
  strip "$STAGE/bin/meridianops-gateway" || true
fi
cp "$CFG_EXAMPLE" "$STAGE/config/gateway-config.toml.example"
cp -a "$PORTAL_DIST"/. "$STAGE/portal/"

{
  echo "package=${PACKAGE_NAME}"
  echo "label=${PACKAGE_LABEL}"
  echo "git=$(git rev-parse HEAD 2>/dev/null || echo unknown)"
  echo "ref=${GITHUB_REF:-}"
  echo "built_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "$STAGE/BUILD.txt"

cat > "$STAGE/README.txt" <<'EOF'
MeridianOps Linux 包

内容
  bin/meridianops-gateway     网关二进制
  portal/                     门户静态文件（Nginx 托管）
  config/gateway-config.toml.example

启动
  1. 复制 example 为 gateway-config.toml，填 MySQL / JWT 等
     生产用环境变量覆盖：MERIDIANOPS_DB_URL、MERIDIANOPS_JWT_SECRET
  2. ./bin/meridianops-gateway --config gateway-config.toml
  3. Nginx 托管 portal/，把 /api 反代到网关 8800

Ubuntu 包在 glibc / Ubuntu runner 上编译；
麒麟包在 hxsoong/kylin:v10-sp3 容器内编译网关，尽量贴近银河麒麟 V10 SP3。
EOF

mkdir -p dist
tar -C dist -czf "dist/${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"
ls -lh "dist/${PACKAGE_NAME}.tar.gz"
echo "staged ${PACKAGE_NAME} (${PACKAGE_LABEL})"
