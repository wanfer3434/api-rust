#!/bin/bash

echo "🛑 matando api-rust..."
pkill api-rust || true

echo "🧹 liberando puerto..."
sleep 1

echo "📦 cargando .env..."
set -a
source .env
echo "KEY EN RUN.SH = ${BINANCE_API_KEY:0:5}"
set +a

echo "🔍 verificando variables..."
echo "KEY=${BINANCE_API_KEY:0:5}..."

echo "⚙️ compilando..."
cargo build --release

echo "🚀 ejecutando..."
./target/release/api-rust
