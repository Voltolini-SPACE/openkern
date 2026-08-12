# OpenKern — Brand Strategy

> Mission OPENKERN-BRAND-01 · sections 3–8
> Base product: commit `d06dddf` · tag `openkern-bootstrap-01`
> Status: WORKING (pre-freeze). Not registered in the brand vault. No publication.

---

## 3. Brand strategy

### 3.1 Purpose — why OpenKern exists

AI agents are being handed the power to *act*: run code, mutate repositories, call
external tools, move data, spend money. The power grew; the authority to wield it did
not. Today that authority is **implicit** — ambient permissions, trusted prompts,
"the agent probably won't do anything bad." OpenKern rejects implicit authority.

OpenKern is the layer where an agent's intent becomes a governed action, or does not
run at all. Authority is explicit. Capabilities are bounded and single-use. Execution
is typed, not a shell. Policy is default-deny and verifiable. Git is transactional
through one chokepoint. And the record of what happened is **evidence**, not the
agent's own claim.

> An agent should never receive authority it was not explicitly granted.

### 3.2 Brand promise

Defensible, engineering-grade, no impossible claims.

**Primary promise:**
> Every action passes through explicit, bounded, verifiable authority — or it does not run.

**Compressed:**
> No implicit authority. No unverified action.

**What we never say** (prohibited claims): "100% secure", "unhackable", "perfect
security", "zero risk", "AI you can finally trust". OpenKern reduces and bounds risk
and makes it auditable. It does not abolish risk, and it will not pretend to.

### 3.3 Positioning & category

OpenKern is **not** a chatbot, an IDE, an LLM wrapper, a generic CLI, or "another agent
framework". It sits *below* those things, as the execution substrate they run on.

Category candidates evaluated:

| Candidate | Verdict |
|---|---|
| AI agent runtime | Rejected — crowded, generic, says nothing about governance |
| AI sandbox | Rejected — collapses the product to one feature |
| Policy engine | Rejected — collapses the product to one feature |
| Execution authority layer | Strong, but "layer" reads passive |
| **Governed Execution Kernel for AI Agents** | **Selected** — "kernel" claims the low-level, protected-core position; "governed" is the differentiator; the phrase is ownable |

**Positioning statement (one line):**
> OpenKern is the governed execution kernel for AI agents: explicit authority,
> bounded capabilities, typed execution, and evidence over claims.

**Frame of reference vs. the field:**

- vs. agent frameworks (LangChain-style): they orchestrate *what to do*; OpenKern
  governs *whether it is allowed to run and proves what ran*.
- vs. sandboxes/containers: they isolate a process; OpenKern governs authority,
  capability, policy, and evidence around every action, fail-closed by default.
- vs. policy engines (OPA-style): policy is one organ inside OpenKern, wired to
  capabilities, typed execution and a transactional Git chokepoint.
- vs. MCP / tool ecosystems: OpenKern treats external systems as **contracts**, not
  as implicit trust. It integrates without becoming subordinate.

---

## 4. Brand attributes

Five attributes. Each carries a visual implication, a verbal implication, and behaviors
that are forbidden.

### 1. Authority (explicit)
Authority is named, granted, and revocable — never ambient.
- Visual: strong structure, defined boundaries, a clear center of control.
- Verbal: declarative, in the active voice. "The kernel grants." "Policy denies."
- Forbidden: mystique, "magic", anthropomorising the agent as trustworthy.

### 2. Boundedness (capability-scoped)
Every grant has an edge. Capabilities are narrow and, where possible, single-use.
- Visual: frames, gates, apertures, clear inside/outside.
- Verbal: precise scope words — "bounded", "one-use", "scoped", "within".
- Forbidden: "unlimited", "full access", "anything you want".

### 3. Determinism (typed, reproducible)
Execution is typed and legible, not an opaque shell. The same input, the same result.
- Visual: grid, alignment, monospace, exact geometry.
- Verbal: specific and testable. Numbers, states, exit codes.
- Forbidden: vague adjectives, hand-waving, "smart"/"intelligent" as a claim.

### 4. Verifiability (evidence over claims)
The record is evidence, and evidence outranks assertion.
- Visual: state labels, checkmarks tied to proofs, audit trails, provenance.
- Verbal: "verified" only when there is a proof; otherwise "unverified".
- Forbidden: claiming a result without the artifact that proves it.

### 5. Restraint (fail-closed, minimal)
When in doubt, deny. Say less. Ship the smallest surface that holds.
- Visual: high contrast, generous negative space, few elements, no decoration-for-its-own-sake.
- Verbal: short, plain, no hype. Silence over noise.
- Forbidden: gradients-as-personality, glow-as-security, buzzword stacking.

---

## 5. Target audience

| # | Persona | Core need | Perceived risk | Relevant promise | Objection | Language to use | Language to avoid |
|---|---|---|---|---|---|---|---|
| A | AI Infrastructure Engineer | A substrate to run agents safely at scale | Ambient authority, blast radius | Explicit, bounded authority per action | "Another layer to operate" | kernel, capability, chokepoint, fail-closed | "magic", "just works", "AI-powered" |
| B | Security Engineer | Enforceable, auditable controls | Unverifiable agent actions | Default-deny + evidence trail | "Marketing security, not real" | threat model, invariant, default-deny, provenance | "unhackable", "100% secure" |
| C | Agent Platform Builder | Governance primitives to build on | Lock-in, opacity | Composable, contract-based integration | "Will it constrain my product?" | primitive, policy, typed execution, SDK | "all-in-one platform" |
| D | Enterprise Architecture / Governance | Control and accountability | Compliance exposure | Verifiable authority + audit | "Can we trust the vendor?" | governance, authority, audit, boundary | consumer hype, emojis, slang |
| E | Open-source Developer | Readable, trustworthy internals | Hidden complexity, bad license | Std-only core, evidence-first docs | "Is it real or a demo?" | open source, reproducible, tests, spec | closed, "enterprise-only" |
| F | Advanced Individual Developer | Safe autonomy for their own agents | Foot-guns, silent failure | Typed execution, no orphans, no shell | "Too heavy for me" | CLI, worktree, mission, sandbox contract | corporate stiffness, jargon soup |

---

## 6. Naming architecture

**Name: OpenKern** — validated, not renamed (rename requires explicit owner authorization).

- Semantics: **Open** (open source, open/inspectable, explicit) + **Kern** (kernel — the
  protected core with the highest authority in a system; also "kern" as in precise
  typographic spacing, a quiet nod to exactness).
- Pronunciation: /ˈoʊpən kɜːrn/ — two syllables, unambiguous in English.
- Memorability: high — "kernel" is a loaded, respected word in systems engineering.
- Differentiation: distinct from "OpenAI", "OpenClaw", "OpenCode" — "Kern" is the
  distinctive root; see §30 originality review.
- International legibility: clean across EN/PT/ES/DE; no unfortunate meanings surfaced.
- Risks: possible confusion with "OpenClaw" (sibling) and with the German word "Kern"
  (= core) — the latter is on-message rather than a problem.

**Name architecture (hypotheses, not decisions):**

```
OpenKern              the mark
OpenKern Core         the kernel crates
OpenKern Runtime      typed execution
OpenKern Policy       policy engine
OpenKern Capabilities capability engine
OpenKern CLI          the `kern` command
OpenKern SDK          embedding surface
OpenKern Cloud        (future, unauthorized)
OpenKern Desktop      (future, unauthorized)
```

CLI binary name: **`kern`** (already the product's CLI). The wordmark is "OpenKern";
the command is `kern`.

---

## 7. Tagline system

24 candidates, five families, scored on clarity / differentiation / credibility /
memorability / global fit / longevity.

### Technical
1. Governed execution kernel for AI agents.
2. Typed execution. Bounded authority.
3. The kernel that governs what your agents run.
4. Explicit authority for agent execution.
5. Default-deny execution for AI agents.

### Institutional
6. Authority, made explicit.
7. Control the runtime. Trust the record.
8. Governance for the agents you let act.
9. Where agent intent becomes governed action.
10. The execution substrate for accountable agents.

### Developer-first
11. No implicit authority.
12. Your agents, on a leash you can inspect.
13. Ask. Allow. Deny. Prove.
14. Run agents like you mean it.
15. Give agents power without giving them the keys.

### Security-first
16. Fail-closed by default.
17. Evidence over claims.
18. Bounded capabilities, verifiable actions.
19. Deny by default. Prove by design.
20. Least authority, every action.

### Minimalist
21. Explicit authority.
22. Govern the run.
23. Kernel for governed execution.
24. Evidence > claims.

### Selection

```
PRIMARY_TAGLINE      : Governed execution for AI agents.
SECONDARY_TAGLINE    : No implicit authority. No unverified action.
TECHNICAL_DESCRIPTOR : Governed Execution Kernel for AI Agents
ONE_LINE_DESCRIPTION : OpenKern is the governed execution kernel for AI agents —
                       explicit authority, bounded capabilities, typed execution,
                       and evidence over claims.
```

(The em-dash appears only in this internal reference line; public copy avoids it.)

Runner-up held in reserve: **"Evidence over claims."** — the shortest expression of the
product's soul, strong for stickers, social, and the security page.

---

## 8. Voice & tone

OpenKern sounds technical, precise, controlled, modern, highly competent, and calm.
Never childish, cyberpunk-edgy, aggressive, buzzword-stuffed, "AI magic", or marketing
without evidence.

**Seven rules**

1. Say the true thing plainly. If it is not proven, do not claim it.
2. Active voice, declarative. "The kernel denies", not "requests may be denied".
3. Specifics beat adjectives. States, numbers, exit codes, invariants.
4. No absolute-security language, ever. Bound and verify; do not promise perfection.
5. Restraint. The shortest version that is still exact wins.
6. No public em-dashes; use a period or a colon. No emoji in product/security surfaces.
7. State is first-class: `ALLOW`, `ASK`, `DENY`, `REFUSED`, `VERIFIED`, `EXECUTED`.

**Applied examples**

- Headline: *Governed execution for AI agents.*
- Docs: *A capability is a bounded, one-use grant of authority. It is checked at the
  chokepoint and consumed on use.*
- Error (CLI): *DENY policy: `net.egress` not permitted for mission `m_4f2`. No action taken.*
- Release note: *G7 — sandbox contract. Capabilities are declared up front; the kernel
  negotiates fail-closed and refuses anything undeclared.*
- Security advisory: *Scope: mission execution. Impact: a declared capability could be
  reused beyond one call. Fix in 0.x. Evidence and repro below.*
- README first line: *OpenKern is the governed execution kernel for AI agents.*
- Social: *Ask. Allow. Deny. Prove.*
- Onboarding: *Start a mission. Declare what it may touch. Run. Read the evidence.*

**Do / Don't**

| Do | Don't |
|---|---|
| "Default-deny." | "Ultra-secure." |
| "Verified: 69 tests." | "Fully tested and safe." |
| "Bounded, one-use capability." | "Powerful permissions." |
| "No action taken." | "Something went wrong." |
