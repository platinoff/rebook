#!/bin/bash
# restart rebook view server (pure MSYS2 bash, no PowerShell): kill old + detached nohup
export PATH="/usr/bin:/bin:/ucrt64/bin:$PATH"
pid=$(netstat -ano | grep ':8090 ' | grep LISTENING | head -1 | awk '{print $NF}')
if [ -n "$pid" ]; then taskkill //PID "$pid" //F; sleep 1; fi
/s/rust/rebook/scripts/run-server.sh
