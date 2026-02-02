# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-02-02

### Bug Fixes

- **security:** Enable Claude OAuth refresh and tighten sandbox access


### CI

- Bump the github-actions group with 3 updates (#15)


### Dependencies

- Bump clap from 4.5.54 to 4.5.56 in the rust-dependencies group (#14)


### Documentation

- Rewrite for clarity and supply chain focus (#18)


### Features

- Add allow_list_dirs for Bun runtime compatibility (#17)

## [0.2.9] - 2026-01-31

### Bug Fixes

- **profile:** Allow Claude Code UID-suffixed tmp dirs and skills (#12)

## [0.2.8] - 2026-01-30

### Bug Fixes

- Update zsh completions and README documentation (#10)


### Documentation

- Update installation to recommend Homebrew over Cargo

## [0.2.7] - 2026-01-29

### Bug Fixes

- **ci:** Simplify homebrew formula update, remove version line

## [0.2.6] - 2026-01-29

### Bug Fixes

- **ci:** Update homebrew script for binary formula

## [0.2.5] - 2026-01-29

### Bug Fixes

- **ci:** Replace mislav action with custom homebrew update script

## [0.2.4] - 2026-01-29

### Bug Fixes

- **ci:** Use full repo path for homebrew tap push-to

- **ci:** Configure homebrew action for direct push to tap

- **ci:** Add GITLEAKS_LICENSE secret to gitleaks action

## [0.2.3] - 2026-01-29

### Bug Fixes

- **ci:** Push directly to homebrew tap instead of forking

- **ci:** Use GitHub App token for release commits

## [0.2.2] - 2026-01-28

### Bug Fixes

- **sandbox:** Harden path validation and error handling (#9)

- **ci:** Skip cargo install if binary already cached


### Refactor

- **release:** Simplify Homebrew tap update workflow (#8)


