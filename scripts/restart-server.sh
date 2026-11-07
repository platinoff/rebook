#!/bin/bash
# restart rebook view server (pure MSYS2 bash, no PowerShell): kill old + detached nohup
export PATH="/c/Users/plati/.cargo/bin:/usr/bin:/bin:/ucrt64/bin:$PATH"
pid=$(netstat -ano | rg ':8090\s' | rg 'LISTENING' | head -1 | awk '{print $NF}')
if [ -n "$pid" ]; then taskkill //PID "$pid" //F; sleep 1; fi
/s/rust/rebook/scripts/run-server.sh
