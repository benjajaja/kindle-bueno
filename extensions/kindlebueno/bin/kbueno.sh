#!/bin/sh

case "$1" in
  start)
    # stop framework
    cd /mnt/us
    killall kindle-bueno
    nohup ./kindle-bueno >> ./kbueno.log 2>&1 &
    lipc-set-prop -i com.lab126.powerd preventScreenSaver 1
    lipc-set-prop com.lab126.framework disable 1
    ;;
  stop)
    killall kindle-bueno
    lipc-set-prop -i com.lab126.powerd preventScreenSaver 0
    # start framework
    lipc-set-prop com.lab126.framework disable 0
    ;;
  ss)
    lipc-set-prop -i com.lab126.powerd preventScreenSaver 1
    ;;
  *)
    echo "Usage: $0 {start|stop|ss}"
    exit 1
    ;;
esac
