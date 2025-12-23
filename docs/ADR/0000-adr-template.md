# ADR [000X]: [Short Title of the Decision]

**Status:** Proposed | Accepted | Superseded
**Date:** 2025-XX-XX
**Author:** [Your Name]

## 1. Context (The "Why")
What is the problem we are solving? 

> *Example: The current Jito integration is throwing PermissionDenied errors because the Searcher Identity isn't whitelisted.*

## 2. Decision
What specific action are we taking?

> *Example: We are implementing a dedicated Keypair for the Jito Searcher and submitting it to the whitelist form.*

## 3. Rationale (The "Proof")
Why is this the best way? Mention any "Measure Twice" research here.
* Avoids "reinventing the wheel" by using Jito's standard auth flow.
* Ensures zero dead code by wiring the new keypair directly into the `.env` setup.

## 4. Consequences
* **Positive:** Clearer logs, successful authentication.
* **Negative/Trade-offs:** 24-48 hour wait time for whitelisting.

## 5. Wiring Check (No Dead Code)
- [ ] Logic implemented in `src/jito_client.ts`
- [ ] Variables added to `.env`
- [ ] Old unused keys deleted/archived
