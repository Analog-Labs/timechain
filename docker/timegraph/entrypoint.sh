#!/usr/bin/env bash
/wait
echo "starting..."
cd /
if [ "$GENESIS" != "" ]; then
  echo "do GENESIS on DATABASE"
  /timegraph genesis --yes
fi
echo "do timegraph job"
/timegraph $@

