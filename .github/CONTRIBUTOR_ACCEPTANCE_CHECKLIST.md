# External Contributor Acceptance Checklist

This document outlines the requirements that all external contributors must complete before submitting a pull request to the Nym repository. Following this checklist ensures code quality, maintainability, and alignment with project standards.

## Code Quality and Compilation

All contributions must compile successfully without warnings or errors across the supported platforms. Before submitting your pull request, verify that your code compiles cleanly using `cargo build --workspace --all-targets`. The codebase uses the latest Rust version in most placesas specified in the workspace Cargo.toml, and your contribution must be compatible with this requirement. All code must pass `cargo clippy --workspace --all-targets -- -D warnings` without any clippy warnings or lints. This means zero tolerance for clippy suggestions, unused imports, dead code, or any other code quality issues that clippy would flag. Additionally, all code must be properly formatted using `cargo fmt --all`, and the CI pipeline will reject any pull request that fails the formatting check.

## Testing Requirements

Every contribution must include appropriate test coverage for the functionality being added or modified. For new features, include unit tests that verify the core behavior and edge cases. For bug fixes, include regression tests that prevent the issue from reoccurring. All existing tests must continue to pass, and you must run `cargo test --workspace` locally to verify this before submission. Integration tests should be included when the contribution affects interactions between multiple components or systems. Test coverage should demonstrate that the code behaves correctly under both normal and error conditions, and any error handling paths should be explicitly tested.

## Documentation and Evidence

For user-facing features or significant changes, provide clear documentation of the functionality. This includes code comments explaining complex logic, updated README files if the contribution affects setup or usage instructions, and inline documentation for public APIs. When the contribution involves UI changes, user flows, or visual modifications, include screenshots or screen recordings that demonstrate the feature working as intended. These visual artifacts should show the complete user journey, including any error states or edge cases that users might encounter. For backend or infrastructure changes, provide diagrams or written descriptions of the architecture changes and how they integrate with existing systems.

## Component Impact Analysis

Clearly document which components, modules, or subsystems your contribution touches. This includes listing all files modified, added, or removed, and explaining the scope of changes within each file. Describe how your changes interact with existing code, including any dependencies you've added or modified, and explain why these dependencies are necessary. If your contribution affects multiple areas of the codebase, provide a clear explanation of how these changes work together and why they were implemented as a cohesive unit rather than separate contributions.

## Functional Benefits and Justification

Articulate the specific benefits your contribution provides to the project. Explain what problem it solves, who benefits from the change, and how it improves the user experience or system functionality. If the contribution addresses a specific issue or feature request, reference the relevant issue number and explain how your implementation fulfills the requirements. For performance improvements, include benchmarks or metrics that demonstrate the enhancement. For new features, explain the use case and how it fits into the broader project goals. If the contribution refactors existing code, explain why the refactoring was necessary and what improvements it brings in terms of maintainability, readability, or performance.

## Workflow Compliance

Your contribution must align with all project workflows and CI/CD requirements. This means your code must pass all automated checks in the GitHub Actions workflows, including build verification, linting, formatting, and test execution. The CI pipeline runs `cargo clippy` with `-D warnings` on all platforms, and any warnings will cause the build to fail. Ensure that your local development environment matches the CI environment as closely as possible, and run the same commands locally that the CI pipeline executes. If your contribution affects areas covered by specialized workflows such as contract compilation, WASM builds, or platform-specific builds, verify that those workflows also pass successfully.

## Code Review Readiness

Before marking your pull request as ready for review, ensure that all items in this checklist are complete. Your pull request description should reference this checklist and confirm that each requirement has been met. Include links to any external documentation, screenshots, or test results that support your contribution. Be prepared to answer questions about design decisions, implementation choices, and alternative approaches that were considered. The code should be self-explanatory where possible, with clear variable names, function names, and structure that make the intent obvious to reviewers who may not be familiar with the specific area of the codebase you've modified.

## Acceptance Criteria Summary

In summary, your contribution is ready for review when it compiles without warnings, passes all clippy checks with zero tolerance for warnings, includes comprehensive tests, provides clear documentation and visual evidence where applicable, clearly explains component impacts and functional benefits, complies with all workflow requirements, and is presented in a manner that facilitates efficient code review. Meeting these standards ensures that external contributions maintain the high quality bar expected in the Nym codebase and reduces the review burden on maintainers while preventing issues from reaching production.

