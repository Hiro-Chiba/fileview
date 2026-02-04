# fileview Product Philosophy

Updated: 2026-02-04

## North Star

`fileview` is not trying to win by becoming a generic "everything file manager."

It must win as:

- the best companion for AI-driven development
- in the remaining ~20% terminal space when tools like Claude Code/Codex CLI occupy most of the screen

## Non-Negotiables

1. Optimize for narrow terminals first.
2. Prioritize AI workflow speed over feature vanity.
3. Keep zero-config onboarding.
4. Ship in small fast loops with measurable score impact.
5. Preserve reliability: no destructive surprises.

## What "Winning" Means

- A developer can open `fileview` and become AI-productive in under 3 minutes.
- Context gathering for AI review/debug is faster than manual shell workflows.
- In split terminals, `fileview` remains readable and useful.
- Release quality and docs clarity build trust over time.

## Decision Gates (Only escalate major decisions)

Escalate to human owner only when one of these is true:

- Breaking change in CLI/MCP API
- Potential data-loss risk
- Stable release cut decision
- Large architecture rewrite
- Pricing/cost/legal/security policy change

Everything else should be executed automatically in the loop.

## Execution Loop

1. Market research
2. Implement smallest high-impact improvement
3. Run quality/security checks
4. Compare score delta in `score.md`
5. Publish docs/history updates
6. Repeat

## Anti-Drift Reminder

When in doubt, choose the option that improves:

- narrow-screen AI workflow quality
- time-to-context for reviews/debugging
- trust signals (stability, docs, release hygiene)
