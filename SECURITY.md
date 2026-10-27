# Security Policy

## Project status

WaterUptime is an experimental prototype. It is not approved for production water-service operations, personal data, or real funds.

## Reporting a vulnerability

Until a private security contact is configured in the public repository, do not publish exploit details or submit sensitive reports containing personal data in public issues. Contact the maintainer privately through the contact channel listed on the GitHub profile and provide a concise description, affected component, reproduction steps, and impact. A dedicated security email or GitHub private vulnerability reporting should be configured before a public release.

## Current security assumptions

- All on-chain state and events are public and may be permanent.
- A hash is not encryption and may be guessable for predictable/low-entropy data.
- A blockchain record shows what an authorised account submitted; it does not independently prove a physical repair or water quality.
- Smart-contract bugs, compromised keys, collusion, false evidence, connectivity problems, and poor governance remain possible.
- No mainnet deployment or real-money workflow is authorised by this specification.

## Before any real-world pilot

Require an independent contract/security review, documented role and key-management model, data-protection review, clear governance and incident handling, human verification procedures, and an accountable implementation partner.
