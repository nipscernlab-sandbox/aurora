#!/bin/sh
# Instala a última release da AURORA (Linux x64, macOS Apple Silicon).
#
#   curl -fsSL https://raw.githubusercontent.com/nipscernlab-sandbox/aurora/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/nipscernlab-sandbox/aurora/main/install.sh | sh -s -- --yes
#
# Baixa o instalador da release, confere o SHA-256 com o SHA256SUMS da
# release, extrai numa pasta provisória e roda o `install`. Os argumentos
# depois de `sh -s --` vão para o `install` (--yes, --components, --prefix,
# --no-link, --list). AURORA_VERSION=0.2.0 fixa a versão.

set -eu

repo="${AURORA_REPO:-nipscernlab-sandbox/aurora}"

die() {
    echo "Error: $*" >&2
    exit 1
}

case "$(uname -s) $(uname -m)" in
    "Linux x86_64") platform=linux-x64 ;;
    "Darwin arm64") platform=darwin-arm64 ;;
    *) die "No AURORA installer for $(uname -s) $(uname -m) (only Linux x64 and macOS Apple Silicon; on Windows, use install.ps1)" ;;
esac

command -v curl >/dev/null 2>&1 || die "curl is required"
command -v tar >/dev/null 2>&1 || die "tar is required"
if command -v sha256sum >/dev/null 2>&1; then
    sha256="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
    sha256="shasum -a 256"
else
    die "sha256sum or shasum is required to verify the download"
fi

# A última versão: /releases/latest redireciona para /releases/tag/v<versão>.
version="${AURORA_VERSION:-}"
if [ -z "$version" ]; then
    latest=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$repo/releases/latest") ||
        die "Could not look up https://github.com/$repo/releases"
    version=${latest##*/v}
fi
case "$version" in
    "" | */*) die "Could not find the latest version at https://github.com/$repo/releases" ;;
esac

name="aurora-$version-$platform"
base="https://github.com/$repo/releases/download/v$version"

# Fora do /tmp: há distribuições que o montam sem permissão de execução.
# Como root (`| sudo sh`), /var/tmp: o sudo pode manter o HOME do usuário, e
# um ~/.cache criado pelo root ficaria no caminho dele.
if [ "$(id -u)" -eq 0 ]; then
    cache=/var/tmp
else
    cache="${XDG_CACHE_HOME:-$HOME/.cache}"
fi
mkdir -p "$cache"
tmp=$(mktemp -d "$cache/aurora-install.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
trap 'exit 130' INT TERM

echo "Downloading AURORA $version ($platform)"
curl -fL --progress-bar -o "$tmp/$name.tar.gz" "$base/$name.tar.gz" ||
    die "Could not download $base/$name.tar.gz"
curl -fsSL -o "$tmp/SHA256SUMS" "$base/SHA256SUMS" ||
    die "Could not download $base/SHA256SUMS"
(
    cd "$tmp"
    grep " $name.tar.gz\$" SHA256SUMS >"$name.sha256" || die "The release SHA256SUMS does not list $name.tar.gz"
    $sha256 -c "$name.sha256" >/dev/null 2>&1 || die "The SHA-256 of $name.tar.gz does not match the release SHA256SUMS"
)
tar xzf "$tmp/$name.tar.gz" -C "$tmp"

# Com `curl | sh`, a entrada padrão é o pipe: a instalação guiada lê o
# teclado pelo terminal. Sem terminal, o install pede --yes.
if [ -t 0 ]; then
    "$tmp/$name/install" "$@"
elif (: </dev/tty) 2>/dev/null; then
    "$tmp/$name/install" "$@" </dev/tty
else
    "$tmp/$name/install" "$@"
fi
