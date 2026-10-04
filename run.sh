#!/bin/bash

echo "🛑 matando api-rust..."
pkill -f "/home/javier/api-rust/target/release/api-rust" || true

echo "🧹 liberando puerto..."
sleep 1

echo "📦 cargando .env..."
set -a
source .env
set +a

echo "🔍 verificando variables..."
echo "DB_URL=${DATABASE_URL}"
echo "BASE_URL=${BASE_URL}"

echo "⚙️ compilando..."
cargo build --release || exit 1

echo "🚀 ejecutando..."

nohup ./target/release/api-rust > api-rust.log 2>&1 &

echo $! > api-rust.pid

sleep 2

if kill -0 "$(cat api-rust.pid)" 2>/dev/null; then
    echo "🟢 api-rust ejecutándose. PID: $(cat api-rust.pid)"
    echo "📄 Logs: ~/api-rust/api-rust.log"
else
    echo "🔴 api-rust no pudo iniciar."
    echo "📄 Últimos logs:"
    tail -30 api-rust.log
    exit 1
fi
