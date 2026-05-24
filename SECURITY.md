# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately using GitHub's
["Report a vulnerability"](https://github.com/MonumentalSystems/orleans-rust-client/security/advisories/new)
workflow rather than opening a public issue. We aim to acknowledge reports
within a few business days.

## Deployment guidance

The bridge is a privileged component: it can call any grain its
`IClusterClient` can reach. Treat it as part of your trusted backend and deploy
it accordingly.

- **Do not expose the bridge directly to the public internet.** It performs no
  authentication of its own.
- **Use transport security and authentication in production.** Terminate TLS
  (and ideally mTLS) at the bridge or an adjacent proxy, and authenticate
  callers with mTLS, a JWT-validating reverse proxy, or a network policy that
  only admits trusted clients. The Rust client supports TLS via the `tls` cargo
  feature (`TlsConfig` with a custom CA, mutual TLS, or public roots) and can
  attach auth headers to every request —
  `OrleansClient::builder(url).bearer_token(token)` / `.api_key(...)` /
  `.metadata(...)` — for a proxy to validate.
- **Co-locate the bridge with the cluster.** Run it inside the same trust
  boundary as the Orleans silos it talks to.
- **Do not leak internal detail.** `BridgeOptions.IncludeExceptionDetail`
  defaults to `false` so exception messages and stack traces are not returned
  to clients. Enable it only in development.
- **Bound message sizes.** Both the client
  (`max_decoding_message_size` / `max_encoding_message_size`) and the gRPC
  server enforce maximum message sizes; tune them to your payloads rather than
  leaving them unbounded.
- **Set request timeouts.** The client and bridge both apply per-call
  deadlines (default 30s). Keep these conservative.
- **Keep logging clean.** Avoid logging payloads or request-context values that
  may contain sensitive data.

## Supported versions

This project is pre-1.0. Security fixes are applied to the `main` branch and
the most recent release.
