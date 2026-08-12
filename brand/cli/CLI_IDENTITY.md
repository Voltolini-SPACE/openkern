# OpenKern — CLI Identity (specification only; the real CLI is frozen at d06dddf)

> BRAND_G7 partial. This documents how `kern` should speak when its UX is next
> iterated. No code changes in this mission (CODE_MUTATION=PROHIBITED).

## Principles

1. State first: every consequential line begins with a mono, uppercase state word.
2. Color is redundant: states carry a glyph prefix so a colorless terminal loses nothing.
3. No spinners that hide state; progress lines are honest and appendable (CI-safe).
4. Errors say what was attempted, what decided the outcome, and what was NOT done.
5. No emoji. No exclamation marks. The kernel does not celebrate.

## State vocabulary and glyph prefixes

```
ALLOW     [+]   green    action permitted and proceeding
ASK       [?]   amber    action suspended pending explicit authority
DENY      [-]   red      policy verdict: not permitted; nothing ran
REFUSED   [x]   red      sandbox/contract refusal at negotiation time
VERIFIED  [=]   green    evidence checked and matching
EXECUTED  [>]   ink      typed execution completed; see evidence id
BLOCKED   [!]   amber    prerequisite absent; fail-closed hold
```

## Banner (`kern` with no args, TTY only)

```
[>_] openkern
     governed execution for AI agents
     kern <command> --help · docs: (docs URL when public)
```

## Version

```
$ kern --version
kern 0.1.0 (openkern-bootstrap-01, d06dddf)
```

## Examples

Success path:
```
$ kern mission run m_4f2
[+] ALLOW    policy: fs.read ./src (rule 12, first-match)
[>] EXECUTED cargo test · exit 0 · 69 passed · evidence ev_9c31
[=] VERIFIED evidence ev_9c31 matches declaration
```

Ask path:
```
[?] ASK      net.egress api.github.com:443 requested by mission m_4f2
             grant once: kern grant m_4f2 net.egress --once
```

Deny path:
```
[-] DENY     policy: net.egress not permitted for mission m_4f2 (rule 3)
             no action taken
```

Sandbox refusal:
```
[x] REFUSED  capability fs.write /etc undeclared at negotiation
             mission halted fail-closed; nothing executed
```

Fatal:
```
[!] BLOCKED  repository identity mismatch: expected repo_a1, found repo_77
             refusing all writes; see kern doctor
```
