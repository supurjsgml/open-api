#!/bin/bash
set -e

APP_DIR="/home/ubuntu/rest-api"
BIN_NAME="open-api"
PORT_A=${PORT_A:-8080}
PORT_B=${PORT_B:-9080}

cd "$APP_DIR"

if ss -tln | grep -q ":${PORT_A} "; then
    CURRENT_PORT=$PORT_A
    TARGET_PORT=$PORT_B
else
    CURRENT_PORT=$PORT_B
    TARGET_PORT=$PORT_A
fi

echo "[Deploy] Starting new instance on port ${TARGET_PORT}..."
PORT=$TARGET_PORT SPRING_PROFILES_ACTIVE=prod nohup "./${BIN_NAME}" > /dev/null 2>&1 &
NEW_PID=$!

for i in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:${TARGET_PORT}/actuator/health" > /dev/null; then
        echo "[Deploy] Health check passed on port ${TARGET_PORT} (PID: ${NEW_PID})"
        break
    fi
    sleep 1
done

OLD_PID=$(lsof -t -i:"${CURRENT_PORT}" || true)
if [ -n "$OLD_PID" ]; then
    echo "[Deploy] Stopping previous instance on port ${CURRENT_PORT} (PID: ${OLD_PID})..."
    kill -15 "$OLD_PID" || true
fi

echo "[Deploy] Deployment completed successfully."
