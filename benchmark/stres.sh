#!/bin/bash

# ==============================================================================
# KONFIGURASI TARGET PORT & BEBAN TEST
# Ubah daftar port di bawah ini sesuai port 5 container kamu di manifest.toml
PORTS=(8081 8082 8083 8084 8085)

# Parameter untuk 'ab' (Apache Bench)
TOTAL_REQ=500     # Total request per container (5 x 5000 = 25.000 request)
CONCURRENCY=10     # Jumlah koneksi bersamaan per container

# Parameter jika menggunakan 'wrk'
DURATION="15s"
THREADS=2
# ==============================================================================

REPORT_FILE="rps_stress_report_$(date +%Y%m%d_%H%M%S).txt"
TMP_DIR=$(mktemp -d)
DAEMON_LOG="$TMP_DIR/daemon_metrics.log"

cleanup() {
    if [ -n "$MON_PID" ] && kill -0 "$MON_PID" 2>/dev/null; then
        kill "$MON_PID" 2>/dev/null
    fi
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# 1. Cek tool benchmark (ab atau wrk)
if command -v ab >/dev/null 2>&1; then
    TOOL="ab"
elif command -v wrk >/dev/null 2>&1; then
    TOOL="wrk"
else
    echo "[ERROR] Tool 'ab' (apache2-utils) atau 'wrk' belum terpasang!"
    echo "Install dulu dengan perintah: apt update && apt install -y apache2-utils"
    exit 1
fi

# 2. Cek daemon obscatd
DAEMON_PID=$(pgrep -x obscatd | head -n 1)
if [ -z "$DAEMON_PID" ]; then
    echo "[ERROR] Daemon 'obscatd' tidak ditemukan!"
    exit 1
fi

echo "============================================================"
echo "Memulai Multi-Container RPS Stress Test (${#PORTS[@]} Container)..."
echo "Tool Uji      : $TOOL"
echo "Target Ports  : ${PORTS[*]}"
echo "PID obscatd   : $DAEMON_PID"
echo "============================================================"

# 3. Catat kondisi awal (Idle) daemon sebelum bombardir
IDLE_STATS=$(ps -p "$DAEMON_PID" -o %cpu=,rss=)
IDLE_CPU=$(echo "$IDLE_STATS" | awk '{print $1}')
IDLE_RAM_MB=$(echo "$IDLE_STATS" | awk '{printf "%.2f", $2/1024}')

# 4. Mulai monitor CPU & RAM obscatd setiap 0.1 detik
(
    while kill -0 "$DAEMON_PID" 2>/dev/null; do
        ps -p "$DAEMON_PID" -o %cpu=,rss= >> "$DAEMON_LOG" 2>/dev/null
        sleep 0.1
    done
) &
MON_PID=$!
sleep 0.3

# 5. Bombardir semua port secara paralel (serentak!)
START_TIME=$(date +%s.%N)

for PORT in "${PORTS[@]}"; do
    OUT_FILE="$TMP_DIR/port_${PORT}.out"
    if [ "$TOOL" = "ab" ]; then
        ab -n "$TOTAL_REQ" -c "$CONCURRENCY" "http://127.0.0.1:${PORT}/" > "$OUT_FILE" 2>&1 &
    else
        wrk -t"$THREADS" -c"$CONCURRENCY" -d"$DURATION" "http://127.0.0.1:${PORT}/" > "$OUT_FILE" 2>&1 &
    fi
done

# Tunggu semua serangan selesai
wait
END_TIME=$(date +%s.%N)

sleep 0.3
kill "$MON_PID" 2>/dev/null
wait "$MON_PID" 2>/dev/null

TOTAL_DURATION=$(awk "BEGIN {printf \"%.2f\", $END_TIME - $START_TIME}")

# 6. Hitung metrik daemon selama stress test
POST_STATS=$(ps -p "$DAEMON_PID" -o %cpu=,rss=)
POST_CPU=$(echo "$POST_STATS" | awk '{print $1}')
POST_RAM_MB=$(echo "$POST_STATS" | awk '{printf "%.2f", $2/1024}')

PEAK_CPU=$(awk '{if($1>max) max=$1} END {printf "%.2f", max+0}' "$DAEMON_LOG")
PEAK_RAM_KB=$(awk '{if($2>max) max=$2} END {print max+0}' "$DAEMON_LOG")
PEAK_RAM_MB=$(awk "BEGIN {printf \"%.2f\", $PEAK_RAM_KB/1024}")

# 7. Parse hasil RPS tiap port dan susun tabel
TABLE_ROWS=""
TOTAL_RPS=0
TOTAL_SUCCESS=0
TOTAL_FAILED=0

for PORT in "${PORTS[@]}"; do
    OUT_FILE="$TMP_DIR/port_${PORT}.out"

    if [ "$TOOL" = "ab" ]; then
        RPS=$(grep "Requests per second:" "$OUT_FILE" | awk '{print $4}')
        LATENCY=$(grep "Time per request:" "$OUT_FILE" | head -n 1 | awk '{print $4}')
        COMPLETE=$(grep "Complete requests:" "$OUT_FILE" | awk '{print $3}')
        FAILED=$(grep "Failed requests:" "$OUT_FILE" | awk '{print $3}')

        # Fallback jika port gagal dihubungi / connection refused
        if [ -z "$RPS" ]; then
            RPS="0.00"
            LATENCY="ERR"
            COMPLETE=0
            FAILED=$TOTAL_REQ
            STATUS="DOWN/ERR"
        else
            STATUS="OK"
        fi
    else
        RPS=$(grep "Requests/sec:" "$OUT_FILE" | awk '{print $2}')
        LATENCY=$(grep "Latency" "$OUT_FILE" | awk '{print $2}')
        COMPLETE=$(grep "requests in" "$OUT_FILE" | awk '{print $1}')
        FAILED=$(grep "Socket errors:" "$OUT_FILE" | awk -F'read ' '{print $2}' | awk -F',' '{print $1}')
        FAILED=${FAILED:-0}

        if [ -z "$RPS" ]; then
            RPS="0.00"
            LATENCY="ERR"
            COMPLETE=0
            STATUS="DOWN/ERR"
        else
            STATUS="OK"
        fi
    fi

    TOTAL_RPS=$(awk "BEGIN {printf \"%.2f\", $TOTAL_RPS + $RPS}")
    TOTAL_SUCCESS=$((TOTAL_SUCCESS + COMPLETE))
    TOTAL_FAILED=$((TOTAL_FAILED + FAILED))

    ROW=$(printf "| %-10s | %-12s | %-14s | %-10s | %-10s |" "Port $PORT" "$RPS" "${LATENCY} ms" "$FAILED" "$STATUS")
    TABLE_ROWS="${TABLE_ROWS}${ROW}\n"
done

# Info sistem
CPU_MODEL=$(lscpu | grep "Model name:" | sed 's/Model name:[[:space:]]*//')
TOTAL_RAM=$(free -h | awk '/^Mem:/ {print $2}')

# 8. Cetak laporan ke file .txt
cat << EOF > "$REPORT_FILE"
======================================================================
               OBSCAT MULTI-CONTAINER RPS STRESS REPORT
======================================================================
Tanggal Uji       : $(date +"%Y-%m-%d %H:%M:%S")
CPU Host          : ${CPU_MODEL:-Unknown} (RAM: $TOTAL_RAM)
Tool Benchmark    : $TOOL (Concurrency: $CONCURRENCY per container)
Total Durasi Uji  : ${TOTAL_DURATION} detik
======================================================================

1. HASIL PERFORMA JARINGAN & RPS PER CONTAINER
+------------+--------------+----------------+------------+------------+

| Target     | RPS (req/s)  | Avg Latency    | Failed Req | Status     |
+------------+--------------+----------------+------------+------------+
$(printf "$TABLE_ROWS")+------------+--------------+----------------+------------+------------+

| TOTAL / AGREGAT RPS       | ${TOTAL_RPS} req/s (Total Sukses: $TOTAL_SUCCESS, Gagal: $TOTAL_FAILED)
+---------------------------+-----------------------------------------+

2. STABILITAS DAEMON (obscatd - PID $DAEMON_PID) SAAT DIBOMBARDir
+------------------------------------------+-----------+-----------+

| Kondisi / Fase                           | CPU (%)   | RAM (MB)  |
+------------------------------------------+-----------+-----------+

| Sebelum Stress Test (Idle)               | $(printf "%-9s" "$IDLE_CPU%") | $(printf "%-9s" "$IDLE_RAM_MB") |
| Puncak Saat Bombardir (Peak)             | $(printf "%-9s" "$PEAK_CPU%") | $(printf "%-9s" "$PEAK_RAM_MB") |
| Setelah Stress Test (Post)               | $(printf "%-9s" "$POST_CPU%") | $(printf "%-9s" "$POST_RAM_MB") |
+------------------------------------------+-----------+-----------+

======================================================================
Catatan: Semua container ditembak secara paralel (bersamaan) untuk
menguji ketahanan bridge network, port forwarding, dan isolasi proses.
======================================================================
EOF

echo ""
echo "Stress test selesai! Laporan disimpan ke: $REPORT_FILE"
echo ""
cat "$REPORT_FILE"
