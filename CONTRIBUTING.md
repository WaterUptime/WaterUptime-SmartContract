# Contributing to WaterUptime

Thank you for considering a contribution. WaterUptime is an early-stage open-source experiment; the problem hypothesis and architecture may change after stakeholder research.

## Before starting

- Read `README.md` and the relevant document under `docs/`.
- Check whether an issue is already open.
- For larger work, discuss the approach before implementation.
- Do not claim that a feature is production-ready unless it is implemented, tested, documented, and reviewed.

## Pull requests

Each PR should:
- reference the issue it resolves;
- explain the change and the reason for it;
- include or update tests for state changes and authorisation rules;
- update docs when contract interfaces or assumptions change;
- avoid personal data, secrets, production keys, or unapproved third-party datasets;
- state which commands were run and whether they passed.

## Security-sensitive changes

Authorization, role separation, evidence handling, state transitions, and any future payment functionality require careful review. Do not merge a security-sensitive change solely because the happy-path demo works.

## Scope discipline

Prefer small, complete changes. Do not add AI, tokens, real payments, or new integrations without a documented problem, user need, and design review.
