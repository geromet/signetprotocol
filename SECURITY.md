# Security policy

## Supported versions

| Version | Supported |
|---|---|
| 0.1.0-beta.x | yes (latest beta only) |

## Reporting a vulnerability

Please **do not open a public issue** for security problems.

Use GitHub's private reporting: **Security → Report a vulnerability** in this repository. If that is not available to you, send a direct message to [@kian_cx on X](https://x.com/kian_cx) asking for a private channel; do not include details in public posts.

Please include what you found, how to reproduce it and which version you tested. We will acknowledge the report as soon as we can and keep you updated until it is fixed.

## What is in scope

- The protocol and SDK (`crates/`, `bindings/`).
- The dedicated server (`servidor/`), for example malformed messages that crash it or let a client cheat.

Note: the reference server has **no authentication** in the beta. Do not expose it to the internet unless you accept that anyone can join.
