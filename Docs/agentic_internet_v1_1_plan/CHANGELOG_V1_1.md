# Plan changes 1.0 → 1.1

**Date:** September 5, 2026. This is a revision of the V1 document, not a move of the product to V2.

| Area | Was | Became |
|---|---|---|
| UI | Proposed egui/eframe | Approved Tauri 2 + Rust bridge + independent daemon; U07 security and U08 reviews UX |
| Google attestors | Mandatory independent threshold tied to the committee infrastructure | Voluntary TrustProfile: organizational 1-of-1 or k-of-n; no transport authority appears |
| Trusted networks | Google-specific credential | Provider-neutral credential; Google, Telegram, organizational/site OIDC; no specific state IdP is invented |
| Telegram | Not included | O08 browser/gateway login, holder-bound handoff, secrets outside the desktop |
| Result | Evidence/provenance and client-side validation, emphasis on future verifiers | Explicit ExecutorDeclaration + CustomerValidation, without fake independent attestation |
| Reputation | Mostly future scoring adapters | V1 public review for an accepted order, replies/amendments, distributed storage, and a basic rating |
| Right to review | Not defined | Two-sided AcceptedOrderReceipt; no dependence on result/payment/subsequent consent of the executor |
| Economic agents | Make-or-buy/subcontracting engine in V1 | A04 contract-only; autonomous economic strategies and complex calculations are V2+ |
| Checks | 72 tasks / 44 requirements | 84 tasks / 52 requirements, updated DAG/cards/release gates; previous IDs preserved |

## New packets

O07 — organizational issuer/site gateway; O08 — Telegram. Q01–Q06 — receipt/right, events/replies, distributed publication, rating reducer, privacy/filter policy, adversarial tests. M07 — MCP review tools. U07 — Tauri security boundary; U08 — review/rating UI. X07 — independent end-to-end acceptance of public reviews.

## What is preserved

Rust headless core, E2EE, groups, ten replicas and repair without clients, NAT/relay, transport postage stamps and their backing, single-spend semantics, subsidy schedule, royalty without authority, one existing L2, MCP/A2A, and mandatory test-first. Transport economics was not moved to V2 along with the economic agents.

## Priority and migration

SOURCE_AMENDMENT_2026_09_05.md records the new requirements. All previous R01–R44 and task IDs are preserved; R23 explicitly changed phase. The new R45–R52 extend the matrix. The old full-plan is not required to execute the new revision. One consistent current version is stored in the ZIP; the provenance of the previous archive is recorded by SHA-256.

## Checks of this revision

Before changing the data/validator, 17 revision-contract tests were added; they produced RED on the 1.0 plan. Then the data and validator guardrails were updated. The final commands and logs are in README and *_GREEN.txt. The checks concern the plan only; no product messenger tests were run.
