---
type: feature-product
feature: passkeys
sibling: tech.md
parent: ../../product-auth.md
updated: 2026-01-15
---

# Feature: Passkeys — Product

Passkeys replace passwords for sign-in.

## Scope

| | |
|---|---|
| **Owns** | passkey sign-in |
| **Does not own** | billing |

## MODIFIED Requirements

### Requirement: Sign in (R1)

The system SHALL issue a session on a valid passkey.

#### Scenario: Valid credentials

- **WHEN** a user presents a valid passkey
- **THEN** a session is issued

## REMOVED Requirements

### Requirement: Sign out (R2)
**Reason**: sessions expire on their own.

## ADDED Requirements

### Requirement: Passkey enrolment (R3)

The system MUST let a signed-in user enrol a passkey.

#### Scenario: Enrol

- **WHEN** a signed-in user adds a passkey
- **THEN** the passkey is stored
