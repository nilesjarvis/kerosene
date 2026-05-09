# Security Policy

Kerosene handles trading keys and optional third-party API tokens. Treat all private keys and tokens as highly sensitive.

## Reporting Security Issues

Do not publish private keys, API tokens, wallet-private material, or exploit details in public issues.

Until a dedicated private disclosure channel exists, open a minimal public issue that says you have a security report without including sensitive details, or contact a maintainer through the repository owner profile.

## Secret Handling

- Never commit config files containing private keys or API tokens.
- Prefer OS keychain storage where available.
- If using encrypted config storage, choose a strong password.
- If any key or token is exposed, revoke or rotate it immediately.

## Supported Versions

This project is pre-1.0. Security fixes target the latest public release and current main branch once the project is published.
