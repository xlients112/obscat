#!/usr/bin/env sh
set -eu

# ==========================================
# KONFIGURASI REPO & VERSI
# ==========================================
OBSCAT_REPO="xlients112/obscat"
YOUKI_REPO="youki-dev/youki"
OBSCAT_VERSION="v0.1.2"
YOUKI_VERSION="0.7.0"
INSTALL_DIR="/usr/local/bin"

echo "Obscat container engine binary installer"
echo "VERSION: v0.1.2-pre-testes"
echo "read docs on github: https://github.com/${OBSCAT_REPO}"

# ==========================================
# 1. CEK AKSES ROOT / SUDO
# ==========================================
if [ "$(id -u)" -ne 0 ]; then
    if command -v sudo >/dev/null 2>&1; then
        SUDO="sudo"
        echo "Using sudo..."
    else
        echo "Error: Make sure your have sudo first." >&2
        exit 1
    fi
else
    SUDO=""
fi

# ==========================================
# 2. CEK DEPENDENSI DASAR (curl & tar)
# ==========================================
for cmd in curl tar; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Error: '$cmd' Not found. install $cmd first." >&2
        exit 1
    fi
done

# ==========================================
# 3. DETEKSI ARSITEKTUR OTOMATIS
# ==========================================
RAW_ARCH="$(uname -m)"
case "$RAW_ARCH" in
    x86_64|amd64)
        ARCH="x86_64"
        ;;
    aarch64|arm64)
        ARCH="aarch64"
        ;;
    *)
        echo "Error: Arch '$RAW_ARCH' is not compatible (only x86_64 and aarch64)." >&2
        exit 1
        ;;
esac

echo "Arch detection: $ARCH"

# ==========================================
# 4. PERSIAPAN DIREKTORI SEMENTARA & CLEANUP
# ==========================================
TMP_DIR="$(mktemp -d)"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

# ==========================================
# 5. DOWNLOAD & EKSTRAK OBSCAT
# ==========================================
OBSCAT_TAR="obscat-${ARCH}-linux-gnu.tar.gz"
OBSCAT_URL="https://github.com/${OBSCAT_REPO}/releases/download/${OBSCAT_VERSION}/${OBSCAT_TAR}"

echo "Downloading obscat ${OBSCAT_URL}..."
curl -fL --progress-bar "$OBSCAT_URL" -o "${TMP_DIR}/${OBSCAT_TAR}"

echo "Unzip obscat to ${INSTALL_DIR}..."
$SUDO tar -xzf "${TMP_DIR}/${OBSCAT_TAR}" -C "$TMP_DIR"
$SUDO install -m 755 "${TMP_DIR}/obscat" "${INSTALL_DIR}/obscat"

# ==========================================
# 6. DOWNLOAD & EKSTRAK YOUKI
# ==========================================
# Format nama rilis resmi youki: youki-<versi>-<arch>-gnu.tar.gz
YOUKI_TAR="youki-${YOUKI_VERSION}-${ARCH}-gnu.tar.gz"
YOUKI_URL="https://github.com/${YOUKI_REPO}/releases/download/v${YOUKI_VERSION}/${YOUKI_TAR}"

echo "Downloading youki v${YOUKI_VERSION} dari ${YOUKI_URL}..."
curl -fL --progress-bar "$YOUKI_URL" -o "${TMP_DIR}/${YOUKI_TAR}"

echo "Unzip youki to ${INSTALL_DIR}..."
$SUDO tar -xzf "${TMP_DIR}/${YOUKI_TAR}" -C "$TMP_DIR"
$SUDO install -m 755 "${TMP_DIR}/youki" "${INSTALL_DIR}/youki"

# ==========================================
# 7. VERIFIKASI INSTALASI
# ==========================================
echo ""
echo "Instalation complete! Binary file:"
echo "TIPS: try obscat daemon-start"
