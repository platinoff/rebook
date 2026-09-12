#!/bin/bash
# rebook view server: detached via nohup (no console window to close)
export PATH="/c/Users/plati/.cargo/bin:/usr/bin:/bin:/ucrt64/bin:$PATH"
cd /s/rust/rebook || exit 1
if netstat -ano | grep ':8090 ' | grep -q LISTENING; then
  echo "rebook already listening on 8090"
  exit 0
fi
nohup ./target/debug/rust_book.exe view >/dev/null 2>&1 &
sleep 1
if netstat -ano | grep ':8090 ' | grep -q LISTENING; then
  echo "rebook live on http://127.0.0.1:8090/"
else
  echo "FAILED to bind 8090" >&2
  exit 1
fi
