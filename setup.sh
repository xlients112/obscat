#!/usr/bin/env bash

# Exit segera jika ada perintah yang gagal
set -eo pipefail

# 1. KONFIGURASI LOG & LOKASI UTAMA
LOG_DIR="/var/log/xserved"
LOG_FILE="${LOG_DIR}/xserve-setup.log"
BIN_DIR="/usr/local/bin"
SYS_BIN_DIR="/usr/bin"
XSERVE_VERSION="0.5.0"
XSERVED_VERSION="0.5.0"
YOUKI_VERSION="0.7.0"

# Memastikan folder log instalasi terbentuk terlebih dahulu
mkdir -p "$LOG_DIR"
touch "$LOG_FILE" 2>/dev/null || true

log_info() {
    echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] \e[32m[INFO]\e[0m $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] \e[31m[ERROR]\e[0m $1" | tee -a "$LOG_FILE" >&2
}

log_info "Started to setup xserve..."


# 2. VALIDASI HAK AKSES ROOT
if [ "$EUID" -ne 0 ]; then
   log_error "This script need a ROOT user ( try sudo ./setup.sh)!"
   exit 1
fi

# Deteksi Arsitektur Mesin
ARCH=$(uname -m)
log_info "architecture detection: $ARCH"

# =========================================================================
# 📦 3. BLOK VARIABEL REPO GITHUB (Sesuai Struktur github.com/xlients-dev/xserve)
# =========================================================================
# Format nama file target rilis Anda: xserve-x86_64, xserved-aarch64, dll.

# --- PILIHAN A: LINK UNTUK DAEMON SERVER (xserved) ---
URL_XSERVE_X86_64="https://gitlab.com/xlients-dev/xserve/-/releases/v0.5.0/downloads/xserve-x86_64-v${XSERVE_VERSION}.tar.gz"
URL_XSERVE_AARCH64="https://gitlab.com/xlients-dev/xserve/-/releases/v0.5.0/downloads/xserve-aarch64-v${XSERVE_VERSION}.tar.gz"

# --- PILIHAN B: LINK UNTUK CLI CLIENT (xgithub.
URL_XSERVED_X86_64="https://gitlab.com/xlients-dev/xserved/-/releases/v0.5.0/downloads/xserved-x86_64-v${XSERVED_VERSION}.tar.gz"
URL_XSERVED_AARCH64="https://gitlab.com/xlients-dev/xserved/-/releases/v0.5.0/downloads/xserved-aarch64-v${XSERVED_VERSION}.tar.gz"

# =========================================================================
# 🔄 4. PROSES PENENTUAN LINK SIFATNYA OTOMATIS BERDASARKAN ARCH
# =========================================================================
TARGET_XSERVED_URL=""
TARGET_XSERVE_URL=""

if [ "$ARCH" = "x86_64" ]; then
    TARGET_XSERVED_URL="$URL_XSERVED_X86_64"
    TARGET_XSERVE_URL="$URL_XSERVE_X86_64"
elif [ "$ARCH" = "aarch64" ]; then
    # Menggunakan binary aarch64 universal untuk Linux & Termux universal
    TARGET_XSERVED_URL="$URL_XSERVED_AARCH64"
    TARGET_XSERVE_URL="$URL_XSERVE_AARCH64"
elif [[ "$ARCH" == *"arm"* ]]; then
    TARGET_XSERVED_URL="$URL_XSERVED_ARMV7"
    TARGET_XSERVE_URL="$URL_XSERVE_ARMV7"
else
    log_error "machine architecture '$ARCH' Not yet supported."
    exit 1
fi

# =========================================================================
# ⚙️ 5. INSTALASI DEPENDENSI DAN LINK UNDUHAN
# =========================================================================

# A. Periksa / Pasang perkakas inti Linux (ip, nsenter)
log_info "Inspecting tools kernel utility (ip, nsenter)..."
if ! command -v ip &> /dev/null || ! command -v nsenter &> /dev/null; then
    log_info "downloaded utilites via package manager..."
    if command -v apt-get &> /dev/null; then
        apt-get update -y && apt-get install -y iproute2 util-linux curl tar libseccomp-dev
    elif command -v apk &> /dev/null; then
        apk add --no-cache iproute2 util-linux curl tar libseccomp
    elif command -v dnf &> /dev/null; then
        dnf install -y iproute2 util-linux curl tar libseccomp
    else
        log_error "uknown package manager, install 'iproute2' and 'util-linux' manualy in your system."
        exit 1
    fi
fi

# B. Unduh dan Pasang Low-Level Runtime (Youki OCI)
if ! command -v youki &> /dev/null; then
    log_info "Installing low-level runtime (youki)..."
    
    # Menyesuaikan URL GitHub Release berdasarkan arsitektur (ARCH)
    if [ "$ARCH" = "aarch64" ]; then
        YOUKI_URL="https://github.com/youki-dev/youki/releases/download/v${YOUKI_VERSION}/youki-${YOUKI_VERSION}-aarch64-musl.tar.gz"
    else
        YOUKI_URL="https://github.com/youki-dev/youki/releases/download/v${YOUKI_VERSION}/youki-${YOUKI_VERSION}-x86_64-musl.tar.gz"
    fi
    
    log_info "Downloading from: $YOUKI_URL"
    
    # Mengunduh, mengekstrak biner 'youki' saja, dan menyimpannya ke $BIN_DIR
    if curl -sSfL "$YOUKI_URL" | tar -xzv -C "$BIN_DIR" youki 2>>"$LOG_FILE"; then
        chmod +x "$BIN_DIR/youki"
        log_info "✓ Youki runtime has been successfully installed at $BIN_DIR/youki."
    else
        log_error "Failed to install Youki. Please install it manually or check the Youki GitHub repository."
        exit 1
    fi
fi


# C. Unduh dan Ekstrak Biner Daemon (xserved)
log_info "Downloading and extracting daemon server from: $TARGET_XSERVED_URL"

# Mengunduh dan langsung mengekstrak ke /usr/local/bin tanpa menyimpan file arsip
if curl -sSfL "$TARGET_XSERVED_URL" | tar -xz -C "$BIN_DIR" 2>>"$LOG_FILE"; then
    # Memastikan file biner daemon hasil ekstrak memiliki izin eksekusi
    chmod +x "$BIN_DIR/xserved"
    log_info "✓ Daemon 'xserved' has been successfully extracted and installed at $BIN_DIR/xserved"
else
    log_error "Failed to download or extract 'xserved' daemon on architecture $ARCH. You can download it from GitLab and install it manually."
    exit 1
fi


# D. Unduh dan Ekstrak Biner CLI Klien (xserve)
log_info "Downloading and extracting client from: $TARGET_XSERVE_URL"

# Karena xserve sekarang berupa tar.gz, prosesnya disamakan dengan daemon menggunakan ekstrak tar
if curl -sSfL "$TARGET_XSERVE_URL" | tar -xz -C "$BIN_DIR" 2>>"$LOG_FILE"; then
    # Memastikan file biner klien hasil ekstrak memiliki izin eksekusi
    chmod +x "$BIN_DIR/xserve"
    log_info "✓ Client 'xserve' has been successfully extracted and installed at $BIN_DIR/xserve"
else
    log_error "Failed to install 'xserve' client on architecture $ARCH. You can download it from GitLab and install it manually."
    exit 1
fi


# E. Penyiapan Struktur Direktori Runtime Daemon
log_info "prepare the directory environment..."
mkdir -p /var/log/xserved
mkdir -p /var/lib/xserved/bundles
mkdir -p /var/run/xserved/youki
mkdir -p /var/lib/xserved/images
log_info "✓ directory enviroment has been success to install."

# =========================================================================
# ⚙️ 6. DEPLOYMENT SYSTEMD SERVICE (Otomatis Jalankan Daemon di Background)
# =========================================================================
log_info "Configure Systemd Service for xserved daemon..."

# Membuat berkas service systemd secara langsung dari script
cat <<EOF > /etc/systemd/system/xserved.service
[Unit]
Description=Xserve Container Engine Daemon
After=network.target

[Service]
Type=simple
ExecStart=${BIN_DIR}/xserved
Restart=always
RestartSec=5
StandardOutput=append:/var/log/xserved/daemon.log
StandardError=append:/var/log/xserved/daemon_error.log

[Install]
WantedBy=multi-user.target
EOF

log_info "reloaded daemon config systemd..."
systemctl daemon-reload

log_info "Enabled xserved daemon on systemd..."
systemctl enable xserved

log_info "Starting daemon server in the backgrond..."
systemctl start xserved

# Berikan jeda 1 detik lalu cek apakah service-nya sukses berjalan
sleep 1
if systemctl is-active --quiet xserved; then
    log_info "✓ Daemon 'xserved' success started in background via Systemd!"
else
    log_error "WARN: Daemon 'xserved' Registered but failed to start automatically. Check with: journalctl -u xserved"
fi


log_info "SETUP XSERVE SUCCESS ON YOUR SYSTEM"

log_info "Log setup saving at: $LOG_FILE"
log_info "then you are ready to try:"
log_info "  1. running Daemon Server : sudo xserved"
log_info "  2. Gunakan Klien CLI      : xserve create --id test --image alpine"
