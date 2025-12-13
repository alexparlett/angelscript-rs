# Sub-Agents

Sub-agents exist to give different work its own context window.
They do NOT exist to roleplay human teams.

## Principle
Each sub-agent:
- Has its own working context (isolated)
- Communicates through structured artifacts (not shared transcripts)
- Returns results via defined schema

## Usage in Claude Code
Invoke with: @agent-name your request

Available agents:
- @code-reviewer - Review code changes
- @test-runner - Run and analyze tests
- @feature-verifier - End-to-end verification

## Anti-Pattern
❌ Shared giant context between agents
❌ Agents "chatting" with each other
❌ Designer Agent, PM Agent, Engineer Agent roleplaying
