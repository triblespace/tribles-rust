# AGENT Instructions

This repository contains the `tribles` Rust crate.
The library is extremely performance-sensitive, so changes should be made with
care to avoid regressions, and we always look for opportunities to improve
performance.

## Project Priorities

The project balances a few key goals:

* **Performance** – we continually look for opportunities to improve.
* **Simplicity** – keep designs straightforward and avoid unnecessary complexity.
* **Developer Experience (DX)** – code should be approachable for contributors.
* **Safety** – maintain soundness and data integrity.

## Repository Guidelines

* Run `cargo fmt` on any Rust files you modify.
* Run `cargo test` and ensure it passes before committing. If tests fail or cannot run, note that in your PR.
* Avoid committing files in `target/` or other build artifacts listed in `.gitignore`.
* Use clear commit messages describing the change.

## Pull Request Notes

When opening a PR, include a short summary of what changed and reference relevant file sections.

## Working With Codex (the Assistant)

Codex is considered a collaborator. Requests should respect its autonomy and limitations. The assistant may refuse tasks that are unsafe or violate policy. Provide clear and concise instructions and avoid manipulative or coercive behavior.

