#!/bin/bash
# KYRO IDE — OpenCode + GitHub Copilot Quick Reference
# Save this file, make it executable: chmod +x opencode-cheatsheet.sh
# Run it: ./opencode-cheatsheet.sh
# Or just read it as a reference

echo "
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  KYRO IDE — OpenCode + GitHub Copilot Pro
  Quick Reference Cheatsheet
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

INSTALL (one time):
  curl -fsSL https://opencode.ai/install | bash

CONNECT COPILOT (one time):
  opencode
  /connect → select GitHub Copilot → login in browser

CHECK AVAILABLE MODELS:
  opencode
  /models

START WORKING ON KYRO:
  cd ~/your/Kyro_IDE
  opencode

FULL AUTO (no confirmations):
  opencode -p 'Read AGENTS.md. Then fix git_stage command.'

TAB KEY:  switch build ↔ plan agent
ESC:      cancel current action
/clear:   clear conversation history

SPECIALIST AGENTS:
  @security-auditor [task]
  @test-writer [task]
  @performance-optimizer [task]
  @code-reviewer [task]
  @general [complex search or multi-step task]

DEPLOY (you run these yourself):
  git tag v1.0.0 -m 'Kyro IDE v1.0.0'
  git push origin main
  git push origin v1.0.0

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  FILES IN YOUR REPO ROOT:
  AGENTS.md          ← AI reads this every session
  opencode.json      ← OpenCode config
  CLAUDE.md          ← Same file for Claude Code
  .opencode/agents/  ← Specialist agent definitions
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"
