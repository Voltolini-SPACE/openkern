# OpenKern — GitHub Brand System

> BRAND_G8. For the FUTURE public repository. Nothing here is published in this
> mission (PUBLICATION=PROHIBITED_UNTIL_OWNER_APPROVAL; repo has no remote).

## Assets

| Asset | Source | Export |
|---|---|---|
| Avatar / org icon | `avatar.svg` | PNG 512, 260 (GitHub min 500 recommended) |
| Social preview | `social_preview.svg` | PNG 1280×640 |
| Favicon | `../logo/refinement/favicon_32.svg`, `favicon_16.svg` | ICO multi-size + PNG |
| App icon concept | avatar tile at 1024 with 20% padding | PNG 1024 |
| OG image (docs site) | social preview minus state strip, plus page title slot | PNG 1200×630 |

## README header (copy direction)

```markdown
<p align="center"><img src="brand/github/avatar.svg" width="88" alt="openkern"></p>

<h1 align="center">openkern</h1>
<p align="center"><b>Governed execution for AI agents.</b><br>
Explicit authority. Bounded capabilities. Typed execution. Evidence over claims.</p>
```

First paragraph after the header (persuasion, honest):

> Your agents can already act. OpenKern decides what they may run, on whose
> authority, inside which boundary, and proves what actually happened. Default-deny
> policy, one-use capabilities, transactional Git through a single chokepoint, and
> typed execution with no shell. If an action is not explicitly permitted, it does
> not run.

## Badges

Only evidence-backed badges. Style `flat-square`, colors from the token palette.

```
build: passing        (CI, real)
tests: 69             (from the frozen validation report)
unsafe: 1 audited     (killpg, documented)
deps: std-only        (8 crates, zero external)
license: (owner decision pending — do not badge until decided)
```

Prohibited badges: stars-for-vanity, "made with love", any security score we do not
compute ourselves.

## Issue / PR templates (visual direction)

- Templates open with a state line the reporter fills: `STATE: BUG | ASK | PROPOSAL`.
- Security template routes to SECURITY.md and forbids public PoC until triage.
- Release notes follow the gate format: `Gx: <name> — <verdict>` per section.

## Repository description (140 chars)

```
Governed execution kernel for AI agents. Default-deny policy, bounded capabilities,
typed execution, transactional Git. Evidence over claims.
```

Topics: `ai-agents` `execution-kernel` `capability-security` `policy-engine`
`sandbox` `rust` `governance` `default-deny`
