# Platform Mechanics

Use this reference when platform behavior matters more than local code shape.

## Runtime Mechanics

- Background and scheduled jobs need stable parameters, user context,
  idempotency, retries, lock strategy, and concise logging.
- Temporary storage and files are lifetime-sensitive. Always identify owner
  session, cleanup path, size limit, and client/server transfer boundary.
- Web client, thin client, server, and background job contexts have different
  API availability. Resolve context before suggesting a platform method.
- Data history, full-text search, and exchange registration are platform
  subsystems with their own background processing and storage cost.

## Security Mechanics

- Authentication, certificates, TLS, OpenID, and external crypto providers
  depend on OS trust, platform version, process user, storage location, and
  server/client boundary.
- Rights checks, privileged mode, RLS, and data separation are part of behavior,
  not only performance constraints.
- Secrets must not move through versioned modules, test fixtures, logs, or final
  assistant output.

## Data Boundaries

- Data separation requires an explicit tenant boundary for reads, writes,
  background jobs, exchange messages, temp data, and reports.
- Integration and exchange code must carry tenant/external ids through
  validation, write, logging, and retry paths.
- Queries that bypass rights or tenant filters need a named justification and a
  verification step.

## Stop Rules

- Do not answer a platform-mechanics question without version, mode, and
  context when those change behavior.
- Do not convert a runtime failure into a code patch until evidence identifies
  the failing subsystem.
- If public MCP `unica` lacks the operation needed to inspect or reproduce the
  behavior, report a Unica MCP contract gap.
