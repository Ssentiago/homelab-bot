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
remote() { ssh "${SSH_OPTS[@]}" "$HOST" "$@"; }

usage() {
    echo "Использование: $0 <режим>"
    echo "  local   собрать локально и залить бинарник на хост"
    echo "  update  запустить на хосте $REMOTE_DIR/$BIN --update (обновление с GitHub)"
    exit 1
}

show_status() {
    log "Статус: $(remote "systemctl is-active $BIN" || true)"
    log "Последние строки журнала:"
    remote "journalctl -u $BIN -n 10 --no-pager" || true
}

deploy_local() {
    log "Сборка: cross build --release --target $TARGET"
    cross build --release --target "$TARGET"
    log "Сборка завершена"

    log "Остановка $BIN на $HOST"
    remote "systemctl stop $BIN"

    # после остановки бот в любом случае запускается обратно
    trap 'on_exit_local' EXIT

    log "Копирование $BIN в $HOST:$REMOTE_DIR/"
    scp "${SSH_OPTS[@]}" "target/$TARGET/release/$BIN" "$HOST:$REMOTE_DIR/"
    log "Копирование завершено"
}

on_exit_local() {
    code=$?
    log "Запуск $BIN на $HOST"
    remote "systemctl start $BIN" || log "ОШИБКА: не удалось запустить $BIN"
    show_status
    if [ "$code" -eq 0 ]; then
        log "Деплой завершён успешно"
    else
        log "Деплой завершился с ошибкой (код $code)"
    fi
}

deploy_update() {
    log "Обновление с GitHub: $REMOTE_DIR/$BIN --update"
    remote "cd $REMOTE_DIR && ./$BIN --update"
    log "Обновление завершено, перезапуск $BIN"
    remote "systemctl restart $BIN"
    show_status
    log "Обновление завершено успешно"
}

case "${1:-}" in
    local)  deploy_local ;;
    update) deploy_update ;;
    *)      usage ;;
esac
