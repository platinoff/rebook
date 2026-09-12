#!/bin/bash
export PATH="/c/Users/plati/.cargo/bin:/usr/bin:/bin:/ucrt64/bin:$PATH"
unset CARGO_TARGET_DIR
cd /s/rust/rebook || exit 1
pid=$(netstat -ano | grep ':8090 ' | grep LISTENING | head -1 | awk '{print $NF}')
if [ -n "$pid" ]; then taskkill //PID "$pid" //F >/dev/null; sleep 1; fi
cargo build 2>&1 | tail -1
nohup ./target/debug/rust_book.exe view >/dev/null 2>&1 &
sleep 2
if netstat -ano | grep ':8090 ' | grep -q LISTENING; then
  echo "rebook live on http://127.0.0.1:8090/"
else
  echo "FAILED to bind 8090" >&2
  exit 1
fi
