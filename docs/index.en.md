# Lintel documentation

[简体中文](index.md) · English · [Repository home](../README.en.md)

Choose a page by what you need to do. First use does not require reading the full specification or learning protocol fields.

## New here

[First use](start-here.en.md) walks through the interface and a synthetic archive task. [Core concepts](concepts.en.md) explains hosts, configuration roots, plans and work packages. For a complete command-line example with explicit checks, use the [CLI trial](quickstart.en.md).

## Work on a specific task

| Goal or problem | Guide |
| --- | --- |
| Protection (including 27-field individual/bulk editing), work preservation, cleanup, recovery and recipes | [Operator guide](operator-guide.en.md) |
| CLI use, standalone candidates or agent handoff | [Agent CLI](agents.en.md) |
| App pages, language and desktop use | [Desktop](desktop.en.md) |
| Extension setup, pairing and post-restart cleanup | [Browser](browser.en.md) |
| SSH hosts, remote runners and disconnected tasks | [Remote](remote.en.md) |
| IPv4/IPv6 tests and host network configuration | [Network](network.en.md) |
| Telemetry targets, channel rules and test results | [Telemetry](telemetry.en.md) |
| Pause or restore a particular systemd service | [Services](services.en.md) |
| App version checks, updating and release preparation | [App updates](app-updates.en.md) |
| Unexpected results, errors or unavailable entry points | [Troubleshooting](troubleshooting.en.md) |

## Understand or develop Lintel

Start with the [architecture/source map](architecture.en.md), then consult [Core / CLI](core.en.md), the [protocol (Chinese)](../contracts/protocol.md) and [verification guide (Chinese)](verification.md). [Visual language (Chinese)](visual-language.md) owns identity and assets; [site hosting (Chinese)](site-hosting.md) covers the separate website deployment process.

For goals, see the [SPEC (Chinese)](SPEC.md) and [product baseline (Chinese)](specs/product-baseline.md). For delivery and verified scope, see [current status (Chinese)](current-state.md). [Baseline evidence](specs/product-baseline-status.md), [acceptance](ACCEPTANCE.md) and [candidate history](candidate-history.md) retain detailed records in Chinese.

Chinese is the canonical product-documentation edition. Paired English pages maintain the same commands, defaults and operational boundaries. Some engineering specifications and acceptance records remain Chinese-only, as labelled above.

This is **0.1.0 Preview**: check the local release-preparation candidate, source capabilities, public downloads and active update channel separately. The [v0.1.0 developer Preview](https://github.com/IndelibleVivi/lintel-cc/releases/tag/v0.1.0) provides manual downloads, without Developer ID signing, notarization or an updater feed.

Page language and direction remain stable. For waiting, partial or `needs_reconciliation` results, keep the original ID and use [Troubleshooting](troubleshooting.en.md) rather than deleting Lintel state or a work package to re-read it.
