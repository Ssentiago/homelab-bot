#!/usr/bin/env bash
set -euo pipefail

HOST="root@homelab.lan"
KEY="$HOME/.ssh/homelab"
TARGET="x86_64-unknown-linux-gnu"
BIN="homelab-bot"
REMOTE_DIR="/opt/homelab-bot"
LOG_FILE="deploy.log"
SSH_OPTS=(-i "$KEY" -o IdentitiesOnly=yes)

exec > >(tee -a "$LOG_FILE") 2>&1

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"; }

on_exit() {
    code=$?
    log "Запуск $BIN на $HOST"
    ssh "${SSH_OPTS[@]}" "$HOST" "systemctl start $BIN" || log "ОШИБКА: не удалось запустить $BIN"
    log "Статус: $(ssh "${SSH_OPTS[@]}" "$HOST" "systemctl is-active $BIN" || true)"
    log "Последние строки журнала:"
    ssh "${SSH_OPTS[@]}" "$HOST" "journalctl -u $BIN -n 10 --no-pager" || true
    if [ "$code" -eq 0 ]; then
        log "Деплой завершён успешно"
    else
        log "Деплой завершился с ошибкой (код $code)"
    fi
}

log "Сборка: cross build --release --target $TARGET"
cross build --release --target "$TARGET"
log "Сборка завершена"

log "Остановка $BIN на $HOST"
ssh "${SSH_OPTS[@]}" "$HOST" "systemctl stop $BIN"

trap on_exit EXIT

log "Копирование $BIN в $HOST:$REMOTE_DIR/"
scp "${SSH_OPTS[@]}" "target/$TARGET/release/$BIN" "$HOST:$REMOTE_DIR/"
log "Копирование завершено"
