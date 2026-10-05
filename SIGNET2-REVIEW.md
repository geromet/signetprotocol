# Signet 2 review — start here

This branch is a small, concrete review of the Signet 2 / Forge draft.

I think the core direction is right:

```text
game
→ shared meaning
→ authoritative server
→ shared meaning
→ game
```

The main thing I think is still missing is that "shared meaning" needs to become
an explicit, deterministic contract before independent translators start
depending on it. This branch is docs-only and based on
`kian-cx/signetprotocol@2ddb136`. It keeps the existing architecture and tightens
the boundaries around it.

## The short version

I would keep Signet 2's architecture, but tighten five boundaries:

1. **Semantic identity.** `fire` is a label, not a complete protocol identity.
   Give semantic items stable versioned identity and a normative definition.
2. **Capability negotiation.** Capabilities are an offer, not final behavior.
   Derive one deterministic session contract from offers + session requirements.
3. **Optional behavior.** "Optional" should not mean "enable it if available."
   Activate in declared preference order and record exact skip reasons.
4. **Resolver failure.** Closed candidate lists are good, but the right answer
   can still be absent. Support `NO_MATCH` / `ABSTAIN`.
5. **Pinned does not mean current.** Pinning makes a decision repeatable. It does
   not prove compatibility after its dependencies change.

That is the core proposal. Everything else is secondary.

## What I think is P0 vs later

### P0 — settle before independent Signet 2 translators

- versioned semantic identity;
- small core + modular profiles;
- deterministic negotiated session contract;
- required vs optional semantics;
- typed fallback;
- preference separated from semantic acceptability;
- deterministic optional-profile activation;
- profile authority classes;
- publisher baseline vs participant overlay;
- `NO_MATCH` / `ABSTAIN`;
- canonical composition / hashing;
- executable cross-language conformance vectors.

### P1 — useful boundaries, secondary

- mapping provenance;
- compatibility freshness;
- effective-profile identity;
- integration mode;
- declared vs enforced permission;
- precise evidence labels.

### P2+ — not blockers

- mature package/update trust infrastructure;
- richer revocation/lifecycle machinery;
- ecosystem governance;
- large validation registries.

## What I am not proposing

- replacing `game → meaning → game`;
- AI in the real-time protocol path;
- one giant ontology;
- every game implementing every profile;
- changing Signet/1 compatibility rules;
- building a heavyweight security stack before Signet 2 can ship;
- treating our research prototype as production behavior.

## Smallest useful test

Freeze approximately: 5–8 intents, 4–6 archetypes, 1 movement/body profile,
1 damage/death lifecycle, 1 optional profile, 1 typed fallback. Build two
materially different translators.

Success:

```text
same semantic definitions
+ same capability offers
+ same session requirements
→ same negotiated contract
```

If independent implementations produce different contracts, the specification is
missing a rule.

## What we actually tested

### L1 — deterministic semantics

Corrected evidence:
`Distributed-Minds/Fleet-Control-Public@f109e15ed1c73b0c76876524ec3c126e34bd63ae`

The cross-language exercise found and fixed:

- non-BMP string-order divergence;
- input-order-dependent rejection reasons;
- missing HASH05 enforcement.

Python and JavaScript now reproduce the same corpus plus adversarial vectors.

### L2 — adapter evidence

Corrected evidence:
`Distributed-Minds/Fleet-Control-Public@ba65b131559a28cf0cb7d3416d492dbd1d91b316`

The first generic adapter abstraction was falsified by the Minecraft RCON
fixture. Original coverage: `NO = 5`, `PARTIAL = 2`. The revised role fixture
reproduces `7/7` coverage using
`HOST / IMPORTER / WORLD / INPUT / PRESENTATION / AUTHORITY`. This is prototype
evidence, not a claim that these six roles are the final API.

### Resolver warning

Top-1 accuracy alone was misleading:

```text
gold_oracle (oracle upper bound, not a resolver):
  top-1 1.000   no-match 1.000   order-stability 1.000

lexical:
  top-1 1.000   no-match 0.000   order-stability 0.167

first-candidate:
  top-1 1.000   no-match 0.000   order-stability 0.000
```

So test `NO_MATCH`, candidate-order perturbation and overrides — not just top-1.

## Read next

If you want more detail:

1. `docs/content/proposals/semantic-contracts.mdx`
2. `docs/content/proposals/translation-profiles.mdx`
3. `docs/content/forge/roadmap.mdx`

Only then, if useful:

4. `docs/content/proposals/translator-trust-and-validation.mdx`

You should not need to read the Fleet-Control research notebook unless you want to
inspect the experiments.

## Validation status

Base upstream: `2ddb136ee941705d3e1c020eaddad93be65026f2`

Validated locally:

- docs build PASS;
- all routes generated;
- changed-page internal links PASS;
- corrected L1 cross-language vectors PASS;
- corrected L2 reproduction PASS;
- local adversarial review: no HIGH finding; independent review is welcome.

Deliberate limitation:

```text
ABI_HOST_HARNESS_PASS != GODOT_RUNTIME_PASS
```

Godot runtime reproduction was blocked, so this branch does not claim it.

## Main question

Should Signet 2 define a deterministic, versioned semantic/session contract
before independent translators are built against the shared vocabulary?

If yes, the exact P0 mechanics can be simplified together.
