#!/bin/bash

# Konfigurasi default (bisa diubah lewat argumen)
TARGET_DIR="${1:-.}"
REPORT_FILE="benchmark_report_$(date +%Y%m%d_%H%M%S).txt"
TMP_LOG=$(mktemp)
TMP_TIME=$(mktemp)

# Pastikan proses monitoring mati kalau skrip dihentikan paksa
cleanup() {
    if [ -n "$MON_PID" ] && kill -0 "$MON_PID" 2>/dev/null; then
        kill "$MON_PID" 2>/dev/null
    fi
    rm -f "$TMP_LOG" "$TMP_TIME"
}
trap cleanup EXIT

# 1. Cek apakah daemon obscatd sedang berjalan
DAEMON_PID=$(pgrep -x obscatd | head -n 1)
if [ -z "$DAEMON_PID" ]; then
    echo "[ERROR] Proses 'obscatd' tidak ditemukan!"
    echo "Jalankan 'obscat daemon-start' terlebih dahulu sebelum menjalankan benchmark."
    exit 1
fi

echo "============================================================"
echo "Memulai Benchmark Obscat..."
echo "Target Folder : $TARGET_DIR"
echo "PID obscatd   : $DAEMON_PID"
echo "============================================================"

# 2. Catat kondisi awal (Idle) daemon sebelum deploy
IDLE_STATS=$(ps -p "$DAEMON_PID" -o %cpu=,rss=)
IDLE_CPU=$(echo "$IDLE_STATS" | awk '{print $1}')
IDLE_RAM_KB=$(echo "$IDLE_STATS" | awk '{print $2}')
IDLE_RAM_MB=$(awk "BEGIN {printf \"%.2f\", $IDLE_RAM_KB/1024}")

# 3. Jalankan background sampler untuk merekam CPU & RAM obscatd tiap 0.1 detik
(
    while kill -0 "$DAEMON_PID" 2>/dev/null; do
        ps -p "$DAEMON_PID" -o %cpu=,rss= >> "$TMP_LOG" 2>/dev/null
        sleep 0.1
    done
) &
MON_PID=$!

# Beri jeda 0.3 detik agar sampler siap
sleep 0.3

# 4. Jalankan perintah deploy dan ukur waktu presisi tinggi
START_TIME=$(date +%s.%N)

if [ -x "/usr/bin/time" ]; then
    /usr/bin/time -f "%M" -o "$TMP_TIME" obscat deploy "$TARGET_DIR"
    DEPLOY_STATUS=$?
    CLI_PEAK_KB=$(cat "$TMP_TIME" 2>/dev/null | tr -d '[:space:]')
else
    obscat deploy "$TARGET_DIR"
    DEPLOY_STATUS=$?
    CLI_PEAK_KB=0
fi

END_TIME=$(date +%s.%N)

# Beri jeda 0.5 detik untuk menangkap lonjakan akhir di daemon, lalu stop sampler
sleep 0.5
kill "$MON_PID" 2>/dev/null
wait "$MON_PID" 2>/dev/null

# 5. Catat kondisi daemon setelah deploy selesai (Post-Deploy)
POST_STATS=$(ps -p "$DAEMON_PID" -o %cpu=,rss=)
POST_CPU=$(echo "$POST_STATS" | awk '{print $1}')
POST_RAM_KB=$(echo "$POST_STATS" | awk '{print $2}')
POST_RAM_MB=$(awk "BEGIN {printf \"%.2f\", $POST_RAM_KB/1024}")

# 6. Hitung metrik total (Durasi, Peak CPU, Peak RAM, Delta RAM)
DURATION=$(awk "BEGIN {printf \"%.3f\", $END_TIME - $START_TIME}")

PEAK_CPU=$(awk '{if($1>max) max=$1} END {printf "%.2f", max+0}' "$TMP_LOG")
PEAK_RAM_KB=$(awk '{if($2>max) max=$2} END {print max+0}' "$TMP_LOG")
PEAK_RAM_MB=$(awk "BEGIN {printf \"%.2f\", $PEAK_RAM_KB/1024}")
RAM_DELTA_MB=$(awk "BEGIN {printf \"%.2f\", $POST_RAM_MB - $IDLE_RAM_MB}")

if [ "$CLI_PEAK_KB" -gt 0 ] 2>/dev/null; then
    CLI_PEAK_MB=$(awk "BEGIN {printf \"%.2f\", $CLI_PEAK_KB/1024}")
else
    CLI_PEAK_MB="N/A (/usr/bin/time tidak terpasang)"
fi

# Ambil info spesifikasi mesin & jumlah container aktif
CPU_MODEL=$(lscpu | grep "Model name:" | sed 's/Model name:[[:space:]]*//')
TOTAL_RAM=$(free -h | awk '/^Mem:/ {print $2}')
KERNEL_VER=$(uname -r)
CONTAINER_COUNT=$(obscat list 2>/dev/null | tail -n +2 | wc -l)

# 7. Tulis hasil laporan ke file .txt dengan tabel rapi
cat << EOF > "$REPORT_FILE"
====================================================================
                  OBSCAT CONTAINER ENGINE BENCHMARK
====================================================================
Tanggal Uji      : $(date +"%Y-%m-%d %H:%M:%S")
Target Deploy    : $(realpath "$TARGET_DIR")
Status Exit      : $([ $DEPLOY_STATUS -eq 0 ] && echo "SUKSES (0)" || echo "GAGAL ($DEPLOY_STATUS)")
--------------------------------------------------------------------
INFORMASI SISTEM
--------------------------------------------------------------------
OS / Kernel      : Linux $KERNEL_VER
CPU              : ${CPU_MODEL:-Unknown}
Total RAM Host   : $TOTAL_RAM
PID obscatd      : $DAEMON_PID
Total Container  : $CONTAINER_COUNT container terdeteksi
====================================================================

1. WAKTU EKSEKUSI & CLI METRICS
+------------------------------------------+-----------------------+

| Metrik                                   | Hasil                 |
+------------------------------------------+-----------------------+

| Total Waktu Deploy (Wall Clock)          | ${DURATION} detik
| Puncak Penggunaan RAM CLI (obscat)       | ${CLI_PEAK_MB} MB
+------------------------------------------+-----------------------+

2. PERFORMA DAEMON (obscatd)
+------------------------------------------+-----------+-----------+

| Kondisi / Fase                           | CPU (%)   | RAM (MB)  |
+------------------------------------------+-----------+-----------+

| Sebelum Deploy (Idle)                    | $(printf "%-9s" "$IDLE_CPU%") | $(printf "%-9s" "$IDLE_RAM_MB") |
| Puncak Saat Deploy (Peak)                | $(printf "%-9s" "$PEAK_CPU%") | $(printf "%-9s" "$PEAK_RAM_MB") |
| Setelah Deploy (Running)                 | $(printf "%-9s" "$POST_CPU%") | $(printf "%-9s" "$POST_RAM_MB") |
+------------------------------------------+-----------+-----------+

| Selisih Kenaikan RAM (Idle -> Running)   |           | +${RAM_DELTA_MB} MB
+------------------------------------------+-----------+-----------+

====================================================================
Catatan: Pengukuran daemon diambil dengan interval sampling 100ms.
====================================================================
EOF

echo ""
echo "Benchmark selesai! Laporan disimpan ke: $REPORT_FILE"
echo ""
cat "$REPORT_FILE"
