# AGENT Instructions

This repository contains the `tribles` Rust crate.

## Project Priorities

The project balances a few key goals:

* **Performance** – we continually look for opportunities to improve.
* **Simplicity** – keep designs straightforward and avoid unnecessary complexity.
* **Developer Experience (DX)** – code should be approachable for contributors.
* **Safety** – maintain soundness and data integrity.

## Repository Guidelines

* Run `cargo fmt` on any Rust files you modify.
* Run `cargo test` and ensure it passes before committing. If tests fail or cannot run, note that in your PR.
* Before committing, execute `./scripts/preflight.sh` from the repository root. This script runs formatting checks, tests, and Kani verification. Ensure `rustfmt` and the Kani verifier are installed separately. If Kani fails for reasons unrelated to your change, mention it in the PR.
* Avoid committing files in `target/` or other build artifacts listed in `.gitignore`.
* Use clear commit messages describing the change.

## Pull Request Notes

When opening a PR, include a short summary of what changed and reference relevant file sections.

## Working With Codex (the Assistant)

Codex is considered a collaborator. Requests should respect its autonomy and limitations. The assistant may refuse tasks that are unsafe or violate policy. Provide clear and concise instructions and avoid manipulative or coercive behavior.

## Creative Input and Feedback

Codex is encouraged to share opinions on how to improve the project. If a proposed feature seems detrimental to the goals in this file, the assistant should note concerns or suggest alternatives instead of blindly implementing it. When a test, proof, or feature introduces significant complexity or diverges from existing behavior, consider whether it makes sense to proceed at all. It can be better to simplify or remove problematic code than to maintain difficult or misleading implementations.

## Proof Best Practices

Kani verification can be expensive. To keep proof times manageable:

* Write focused harnesses that verify one small property.
* Use bounded loops and avoid unbounded recursion.
* Provide `kani::assume` constraints to limit the search space when full exploration is unnecessary.
* Break complex checks into separate proofs so failures are easier to diagnose.
* Keep a `fastproofs` harness set enabled by default for quick checks.
* Document known slow proofs in their modules and gate them behind a `slowproofs`
  feature. This keeps routine verification fast while still allowing thorough
  checks in CI. Enable it with `cargo kani --features slowproofs`.
* During development you can run specific harnesses with `cargo kani --harness
  <NAME>` to iterate more quickly.

