#!/bin/sh

case "$1" in
  start)
    cd /mnt/us
    nohup ./kindle-bueno >> /mnt/us/kbueno.log 2>&1 &
    echo "kindle-bueno started"
    ;;
  stop)
    killall kindle-bueno
    echo "kindle-bueno stopped"
    ;;
  *)
    echo "Usage: $0 {start|stop}"
    exit 1
    ;;
esac

