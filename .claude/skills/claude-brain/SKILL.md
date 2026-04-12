---
name: claude-brain
description: Check Claude Brain status — shows whether hooks, MCP server, and memory are working
user-invocable: true
allowed-tools: Bash, Read
---

# Claude Brain Status Check

Run the following checks and report results:

## 1. Binary exists
!`ls -lh /home/nightness/dev/brainwires-framework/target/release/claude-brain 2>&1 | head -1`

## 2. Hook config
!`python3 -c "import json; d=json.load(open('/home/nightness/dev/brainwires-framework/.claude/settings.local.json')); print('DISABLE_AUTO_COMPACT:', d.get('env',{}).get('DISABLE_AUTO_COMPACT','NOT SET')); print('Hooks:', list(d.get('hooks',{}).keys()))" 2>&1`

## 3. MCP config
!`cat /home/nightness/dev/brainwires-framework/.mcp.json 2>&1`

## 4. Memory stats
!`echo '{}' | /home/nightness/dev/brainwires-framework/target/release/claude-brain hook session-start 2>&1 | head -20`

Report a concise status summary showing what's working and what's not.
