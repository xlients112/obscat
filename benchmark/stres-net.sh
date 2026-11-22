#!/bin/bash

# ==============================================================================
# KONFIGURASI TARGET 1 CONTAINER
# Bisa juga dijalankan lewat argumen: ./stress_single_obscat.sh 8081
TARGET_PORT="${1:-8081}"
TOTAL_REQ=2000      # Total 2.000 request
CONCURRENCY=20      # 20 koneksi bersamaan
# ==============================================================================

REPORT_FILE="single_rps_port${TARGET_PORT}_$(date +%Y%m%d_%H%M%S).txt"
TMP_DIR=$(mktemp -d)
DAEMON_LOG="$TMP_DIR/daemon_metrics.log"
OUT_FILE="$TMP_DIR/ab_result.out"

cleanup() {
    if [ -n "$MON_PID" ] && kill -0 "$MON_PID" 2>/dev/null; then
        kill "$MON_PID" 2>/dev/null
    fi
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# 1. Cek tool 'ab' (Apache Bench)
if ! command -v ab >/dev/null 2>&1; then
    echo "[ERROR] Tool 'ab' belum terpasang! Jalankan: apt update && apt install -y apache2-utils"
    exit 1
fi

# 2. Cek daemon obscatd
DAEMON_PID=$(pgrep -x obscatd | head -n 1)
if [ -z "$DAEMON_PID" ]; then
    echo "[ERROR] Daemon 'obscatd' tidak ditemukan!"
    exit 1
fi

echo "============================================================"
echo "Memulai Single-Container RPS Test..."
echo "Target URL    : http://127.0.0.1:${TARGET_PORT}/"
echo "Beban Uji     : $TOTAL_REQ requests ($CONCURRENCY concurrency)"
echo "PID obscatd   : $DAEMON_PID"
echo "============================================================"

# 3. Catat kondisi awal (Idle) daemon
IDLE_STATS=$(ps -p "$DAEMON_PID" -o %cpu=,rss=)
IDLE_CPU=$(echo "$IDLE_STATS" | awk '{print $1}')
IDLE_RAM_MB=$(echo "$IDLE_STATS" | awk '{printf "%.2f", $2/1024}')

# 4. Monitor CPU & RAM obscatd setiap 0.1 detik
(
    while kill -0 "$DAEMON_PID" 2>/dev/null; do
        ps -p "$DAEMON_PID" -o %cpu=,rss= >> "$DAEMON_LOG" 2>/dev/null
        sleep 0.1
    done
) &
MON_PID=$!
sleep 0.2

# 5. Jalankan benchmark pada 1 port (dengan -k Keep-Alive & batas waktu max 15 detik)
START_TIME=$(date +%s.%N)
ab -k -t 15 -n "$TOTAL_REQ" -c "$CONCURRENCY" "http://127.0.0.1:${TARGET_PORT}/" > "$OUT_FILE" 2>&1
AB_STATUS=$?
END_TIME=$(date +%s.%N)

sleep 0.2
kill "$MON_PID" 2>/dev/null
wait "$MON_PID" 2>/dev/null

TOTAL_DURATION=$(awk "BEGIN {printf \"%.2f\", $END_TIME - $START_TIME}")

# 6. Hitung metrik daemon
POST_STATS=$(ps -p "$DAEMON_PID" -o %cpu=,rss=)
POST_CPU=$(echo "$POST_STATS" | awk '{print $1}')
POST_RAM_MB=$(echo "$POST_STATS" | awk '{printf "%.2f", $2/1024}')

PEAK_CPU=$(awk '{if($1>max) max=$1} END {printf "%.2f", max+0}' "$DAEMON_LOG")
PEAK_RAM_KB=$(awk '{if($2>max) max=$2} END {print max+0}' "$DAEMON_LOG")
PEAK_RAM_MB=$(awk "BEGIN {printf \"%.2f\", $PEAK_RAM_KB/1024}")

# 7. Ambil hasil metrik dari Apache Bench
RPS=$(grep "Requests per second:" "$OUT_FILE" | awk '{print $4}')
LATENCY_MEAN=$(grep "Time per request:" "$OUT_FILE" | head -n 1 | awk '{print $4}')
LATENCY_P95=$(grep "95%" "$OUT_FILE" | awk '{print $2}')
LATENCY_P99=$(grep "99%" "$OUT_FILE" | awk '{print $2}')
COMPLETE=$(grep "Complete requests:" "$OUT_FILE" | awk '{print $3}')
FAILED=$(grep "Failed requests:" "$OUT_FILE" | awk '{print $3}')
TRANSFER_RATE=$(grep "Transfer rate:" "$OUT_FILE" | awk '{print $3}')

if [ -z "$RPS" ]; then
    STATUS_TEXT="GAGAL / PORT TIDAK MERESPONS"
    RPS="0.00"
    LATENCY_MEAN="N/A"
    LATENCY_P95="N/A"
    LATENCY_P99="N/A"
    COMPLETE=0
    FAILED=$TOTAL_REQ
    TRANSFER_RATE="0"
else
    STATUS_TEXT="SUKSES"
fi

# Info sistem
CPU_MODEL=$(lscpu | grep "Model name:" | sed 's/Model name:[[:space:]]*//')
TOTAL_RAM=$(free -h | awk '/^Mem:/ {print $2}')

# 8. Cetak laporan ke file .txt
cat << EOF > "$REPORT_FILE"
====================================================================
              OBSCAT SINGLE-CONTAINER RPS BENCHMARK
====================================================================
Tanggal Uji      : $(date +"%Y-%m-%d %H:%M:%S")
Target Endpoint  : http://127.0.0.1:${TARGET_PORT}/
CPU Host         : ${CPU_MODEL:-Unknown} (RAM: $TOTAL_RAM)
Status Pengujian : $STATUS_TEXT (Durasi: ${TOTAL_DURATION} detik)
====================================================================

1. METRIK PERFORMA HTTP & NETWORK BRIDGE
+------------------------------------------+-----------------------+

| Metrik                                   | Hasil                 |
+------------------------------------------+-----------------------+

| Requests Per Second (RPS)                | ${RPS} req/s
| Total Request Sukses                     | ${COMPLETE} requests
| Total Request Gagal                      | ${FAILED} requests
| Rata-rata Latency (Mean)                 | ${LATENCY_MEAN} ms
| Latency Persentil 95% (p95)              | ${LATENCY_P95:-N/A} ms
| Latency Persentil 99% (p99)              | ${LATENCY_P99:-N/A} ms
| Transfer Rate                            | ${TRANSFER_RATE} KB/sec
+------------------------------------------+-----------------------+

2. STABILITAS DAEMON (obscatd - PID $DAEMON_PID)
+------------------------------------------+-----------+-----------+

| Kondisi / Fase                           | CPU (%)   | RAM (MB)  |
+------------------------------------------+-----------+-----------+

| Sebelum Uji (Idle)                       | $(printf "%-9s" "$IDLE_CPU%") | $(printf "%-9s" "$IDLE_RAM_MB") |
| Puncak Saat Uji (Peak)                   | $(printf "%-9s" "$PEAK_CPU%") | $(printf "%-9s" "$PEAK_RAM_MB") |
| Setelah Uji (Post)                       | $(printf "%-9s" "$POST_CPU%") | $(printf "%-9s" "$POST_RAM_MB") |
+------------------------------------------+-----------+-----------+

====================================================================
EOF

echo ""
echo "Selesai! Laporan disimpan ke: $REPORT_FILE"
echo ""
cat "$REPORT_FILE"
