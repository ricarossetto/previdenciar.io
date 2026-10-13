# Contributing

Please read README.md, docs/LIMITATIONS.md and docs/PRIVACY.md first.

1. Open an issue describing the expected behavior using synthetic data only.
2. Use a small branch and focused changes. Keep API/model contracts stable unless the issue calls for a migration.
3. Add tests for exact cents, source preservation, missing values, blocked outcomes and cancellation/error states.
4. Run cargo test --workspace --locked and, in apps/desktop, npm ci, npm run build and npm test. Run browser tests with an installed Playwright Chromium.
5. Submit a pull request explaining scope, tests, limitations and any changes to calculation rules.

Do not copy another application's source, databases, binaries, icons, fonts or private documents into the project. Do not use a client CNIS as a test fixture. Only submit assets you have permission to distribute and preserve their notices.

By submitting a contribution, you agree to license your project-authored contribution under the repository's MIT license, while retaining applicable third-party notices.
