# ADR-0002: Integer milli-big-blind chip units

Status: Accepted (2026-11-22)

## Decision
Chip amounts are `u64` milli-big-blinds (1 bb = 1000 mbb). Currency amounts are integer minor
units converted to mbb via the stake; inexact conversions are rejected.

## Rationale
Floating-point chip arithmetic breaks pot conservation and makes side-pot / rake results
non-deterministic. 1/1000 bb is finer than any realistic bet sizing or rake increment while
keeping 200bb stacks (200,000 mbb) far from overflow.

## Consequences
Odd-chip splits happen at 1 mbb granularity. Rake rounding increments are configurable and
expressed in either bb or currency.
