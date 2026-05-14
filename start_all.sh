#!/bin/bash
# Скрипт для одновременного запуска сервера и фронтенда

echo "=========================================="
echo "🚀 Запуск Gotiga (Axum Сервер + SvelteKit)..."
echo "=========================================="

# Функция для завершения фоновых процессов при выходе
cleanup() {
    echo ""
    echo "🛑 Остановка всех процессов..."
    kill $SERVER_PID
    exit
}

# Перехватываем Ctrl+C (SIGINT) и SIGTERM
trap cleanup SIGINT SIGTERM

# 1. Запуск backend сервера в фоновом режиме
cd "$(dirname "$0")/src-tauri/server" || exit
echo "⏳ Запускаем Axum сервер на порту 3000..."
cargo run &
SERVER_PID=$!

# 2. Возвращаемся в корень и запускаем frontend
cd ../..
echo "⏳ Запускаем Vite dev server..."
npm run dev

# (Если npm run dev завершится сам по себе, мы тоже сделаем cleanup)
cleanup
