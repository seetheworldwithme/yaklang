#!/usr/bin/env bash
# 通过 Docker 在 Linux amd64 环境中编译 yak，避免 musl/glibc 不兼容问题。
# 前置：需要安装 Docker（macOS 推荐 OrbStack 或 Docker Desktop）。
# 用法：在仓库根目录执行 ./scripts/build_yak_linux_amd64.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

GO_VERSION="$(go version | awk '{print $3}' | sed 's/go//')"
OUT="${OUT:-$REPO_ROOT/build/yak_linux_amd64}"

mkdir -p "$REPO_ROOT/build"

# ---------- 阶段 1：macOS 上生成 embed 资源（不依赖 Linux） ----------

echo "[linux-amd64-build] repos-tag..."
go run common/yak/cmd/yak.go repos-tag -o tags.txt
YAK_TAG="$(cat tags.txt)"
echo "[linux-amd64-build] YAK_TAG=$YAK_TAG"

echo "[linux-amd64-build] generate yakdoc / codec embed blobs..."
go run -gcflags=all="-N -l" common/yak/yakdoc/generate_doc/generate_doc.go common/yak/yakdoc/doc/doc.gob.gzip
go run -gcflags=all="-N -l" common/yak/yaklib/codec/codegrpc/doc/codec_doc.go common/yak/yaklib/codec/codegrpc/codec.gob.gzip

echo "[linux-amd64-build] embed-fs-hash..."
go run common/yak/cmd/yak.go embed-fs-hash --override --all

echo "[linux-amd64-build] gzip-embed tool..."
go build -o "$REPO_ROOT/build/gzip-embed" ./common/utils/gzip_embed/gzip-embed

echo "[linux-amd64-build] gzip embed resources..."
"$REPO_ROOT/build/gzip-embed" -cache --source ./common/ai/aid/aitool/buildinaitools/yakscripttools/yakscriptforai --gz ./common/ai/aid/aitool/buildinaitools/yakscripttools/yakscriptforai.tar.gz --no-embed
"$REPO_ROOT/build/gzip-embed" -cache --source ./common/ai/aid/aireact/skills --gz ./common/ai/aid/aireact/skills.tar.gz --root-path --no-embed
"$REPO_ROOT/build/gzip-embed" -cache --source ./common/coreplugin/base-yak-plugin --gz ./common/coreplugin/base-yak-plugin.tar.gz --root-path --no-embed
"$REPO_ROOT/build/gzip-embed" -cache --source ./common/syntaxflow/sfbuildin/buildin --gz ./common/syntaxflow/sfbuildin/buildin.tar.gz --root-path --no-embed
"$REPO_ROOT/build/gzip-embed" -cache --source ./common/aiforge/buildinforge --gz ./common/aiforge/buildinforge.tar.gz --root-path --no-embed

# ---------- 阶段 2：Docker 内编译 ----------

echo "[linux-amd64-build] building inside Docker (golang:${GO_VERSION}-bookworm)..."
YAK_TAG="$YAK_TAG" docker run --rm \
  -v "$REPO_ROOT:/src" \
  -w /src \
  -e YAK_TAG="$YAK_TAG" \
  golang:${GO_VERSION}-bookworm \
  bash -c '
    set -euo pipefail
    apt-get update -qq && apt-get install -y -qq libpcap-dev > /dev/null 2>&1
    export CGO_ENABLED=1
    export GOOS=linux
    export GOARCH=amd64
    go build -tags gzip_embed \
      -ldflags "-s -w -X \"main.goVersion=$(go version)\" -X \"main.gitHash=$(git show -s --format=%H)\" -X \"main.buildTime=$(git show -s --format=%cd)\" -X \"main.yakVersion=${YAK_TAG}\"" \
      -o /src/build/yak_linux_amd64 -v common/yak/cmd/yak.go
  '

echo "[linux-amd64-build] done: $OUT"
ls -lh "$OUT"
