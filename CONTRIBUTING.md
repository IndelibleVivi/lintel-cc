# Contributing

Use Node.js >=22.12.0, npm >=10, Python 3 and Rust stable. Native macOS builds also require Xcode Command Line Tools. Start with [the development trial](docs/start-here.en.md) and [the architecture map](docs/architecture.en.md).

```sh
cd apps/desktop
npm ci
npm run prepare:locked-dependencies
cd ../..
python3 tests/verify.py --checks app-release-test --require-passed
```

The explicit dependency preparation downloads locked Cargo graphs and rust-docs; notice generation stays offline and fails if required license text is missing. See [verification (Chinese)](docs/verification.md) for selecting checks relevant to your change. Keep mutation tests on disposable synthetic roots, never personal Claude data, browser profiles or live services. Preserve original task IDs and query-only recovery after uncertain execution.

Ordinary reproducible bugs and feature discussions can use repository issues. Include platform, command, expected behavior and sanitized error code. Do not post credentials, work transcripts, private paths or raw state. A dedicated private vulnerability reporting channel has not been established here; do not publish sensitive exploit details in an issue.

Source checks, candidate packaging, actual installation and public release are separate evidence. Do not describe synthetic fixtures as authenticated Claude or clean-machine acceptance. Intentional contributions to project-original code are offered under SUL-1.0; contributions to original documentation/independent diagrams under CC BY-NC-SA 4.0, subject to LICENSING.md exclusions. Contributors retain copyright; this is not a copyright assignment. Disclose third-party origins and governing terms; do not contribute material you cannot authorize.
