---
title: Pre-commit tools
---

Pre-commit tools run a configurable number of checks every time you want to commit some code.
Their purpose is to catch some easily avoidable errors, such as linting or styling errors, before they are pushed to a branch.
The two main pre-commit tools are [`pre-commit`](https://pre-commit.com/) and [`prek`](https://prek.j178.dev/).
Jarl provides a pre-commit hook that works with both of these tools: [`jarl-pre-commit`](https://github.com/etiennebacher/jarl-pre-commit).

## `pre-commit`

Use this in `.pre-commit-config.yaml`:

```yaml
repos:
-   repo: https://github.com/etiennebacher/jarl-pre-commit
    rev: 1.0.0
    hooks:
      - id: jarl-check
```

## `prek`

`prek` can read `.pre-commit-config.yaml` but also has its own format, `prek.toml`.

Use this in `prek.toml`:

```toml
[[repos]]
repo = "https://github.com/etiennebacher/jarl-pre-commit"
rev = "1.0.0"
hooks = [
  { id = "jarl-check" },
]
```

## Choosing the version of Jarl to use

Each release of `jarl-pre-commit` installs the latest version of Jarl available at the time of the release.
Run `pre-commit autoupdate` or `prek autoupdate` to get newer versions of Jarl.
Before `1.0.0`, tags of `jarl-pre-commit` matched Jarl's versions; since `1.0.0`, they are independent (see [`jarl-pre-commit` tags](https://github.com/etiennebacher/jarl-pre-commit/tags)).

To pin a specific version of Jarl, use `additional_dependencies` with `jarl-linter` (available on PyPI from 0.5.0):

```yaml
repos:
-   repo: https://github.com/etiennebacher/jarl-pre-commit
    rev: 1.0.0
    hooks:
      - id: jarl-check
        additional_dependencies: ["jarl-linter==0.6.0"]
```

`autoupdate` doesn't modify `additional_dependencies`, so this pin must be updated manually.

## How Jarl is installed

The hook uses `language: python` and installs the `jarl-linter` package from PyPI, which contains the Jarl binary for your platform.
The binary is downloaded once during environment setup and cached, so no network access is needed when the hook runs. This also works on pre-commit.ci and similar services that only allow network access during setup.

Each `rev` gets its own cached environment. This implies two things:

- you may have projects with `pre-commit` config files that use different versions of Jarl;
- the version of Jarl on the `$PATH` is not affected, meaning that projects that don't use `pre-commit` are not affected by the version of Jarl used in a project with `pre-commit`.
