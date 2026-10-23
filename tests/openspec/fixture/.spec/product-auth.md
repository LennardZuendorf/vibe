---
type: product-topic
parent: product.md
scope: auth
covers: sign-in
updated: 2026-01-15
---

# auth — Product

Sign-in for every user.

## Requirements

### Requirement: Sign in (R1)

The system SHALL issue a session on valid credentials.

#### Scenario: Valid credentials

- **WHEN** a user submits valid credentials
- **THEN** a session is issued

### Requirement: Sign out (R2)

The system SHALL end the session on request.

#### Scenario: Sign out

- **WHEN** the user signs out
- **THEN** the session ends
