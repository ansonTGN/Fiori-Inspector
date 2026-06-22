#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"
mkdir -p runs

echo ""
echo "Fiori Inspector Studio"
echo "──────────────────────"
echo "Arrancando interfaz local..."
echo "Abre: http://127.0.0.1:7820"
echo "Para detener: Ctrl+C"
echo ""

cargo run
