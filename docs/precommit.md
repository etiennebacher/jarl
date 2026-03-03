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
    rev: 0.4.0
    hooks:
      - id: jarl-check
```

## `prek`

`prek` can read `.pre-commit-config.yaml` but also has its own format, `prek.toml`.

Use this in `prek.toml`:

```toml
[[repos]]
repo = "https://github.com/etiennebacher/jarl-pre-commit"
rev = "0.4.0"
hooks = [
  { id = "jarl-check" },
]
```

## Choosing the version of Jarl to use

The `rev` parameter determines the version of Jarl to use. Starting from 0.4.0, all releases of Jarl have a matching release in `jarl-pre-commit`.

When you call `pre-commit` or `prek`, it fetches the corresponding Jarl binary and caches it in `~/.cache/jarl-pre-commit` (for instance `~/.cache/jarl-pre-commit/jarl-0.4.0`). This implies two things:

- you may have projects with `pre-commit` config files that use different versions of Jarl;
- the version of Jarl on the `$PATH` is not affected, meaning that projects that don't use `pre-commit` are not affected by the version of Jarl used in a project with `pre-commit`.
