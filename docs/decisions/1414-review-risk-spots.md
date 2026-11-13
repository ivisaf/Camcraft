## #1414 - Review by named risk spots; bounded review rounds

**Decided.** The worker's draft PR body names 3-5 `Risk spots:`, each with
the reference to compare it with; the `PR-reviewer` compares exactly these
step by step (choosing them itself when the list is missing) before its law
check, and its review says what each spot was compared with. A PR whose
second full review is again `needs-rework` raises its issue's effort label
one step instead of a third rework on the same tier.

**Why.** A sister project with the same builder -> reviewer setup measured
on 2026-11-12 that reviews without named spots sampled and found nothing;
the one real finding came only from named spots. An unbounded
rework/review loop on a too-cheap tier costs a full review build per round.

**Considered and dropped.** A separate reviewer agent for risk spots (one
more dispatch and lock per PR, for what one step of the existing review
does); a hard cap of one review round (the second round often closes a
small finding cheaply - the cap is on the third); having the reviewer
choose the spots always (the worker knows where it was unsure; the
reviewer keeps the fallback).

**Cost to change later.** Text only in AGENTS.md, `work-issue` and
`PR-reviewer`; no code or tooling depends on it.
