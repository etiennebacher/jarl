---
title: Getting started
---

## Installation

See the ["Installation" section](index.md#installation) on the homepage.

## Linting

`jarl check` is the command used to diagnose one or several files.
It takes a path as its first argument, such as `jarl check .` to check all files starting from the current directory.
This command will return a list of diagnostics, one per rule violation.

Jarl comes with a [list of rules](rules.qmd) but not all of them are enabled by default.
You can select or ignore rules either via the [command-line interface (CLI)](reference/cli.md) or in a [configuration file](reference/config-file.md).
The full list of rules and families is available on the [Rules](rules.qmd) page.

See the [By Example](by-example.qmd) page for concrete input/output examples.

## Fixing

It can be tedious to fix rule violations one by one.
Jarl can apply automatic fixes to some diagnostics by passing the argument `--fix`, such as `jarl check . --fix`.

Automatic fixes are distinguished between "safe" and "unsafe":

- **Safe fixes** do not change the behavior of the code when it runs, but improve its readability or performance, for instance by using more appropriate functions (see [`any_is_na`](rules/any_is_na.md)).

- **Unsafe fixes** may change the behavior of the code when it runs.
For example, [`all_equal`](rules/all_equal.md) reports cases such as `!all.equal(x, y)`.
This code is likely a mistake because `all.equal()` returns a character vector and not `FALSE` when `x != y`.
Jarl could fix this to be `!isTRUE(all.equal(x, y))` instead, but this would change the behavior of the code, so it is marked "unsafe".

By default, only safe fixes are applied.
To also apply the unsafe fixes, use `--unsafe-fixes`, e.g. `jarl check . --fix --unsafe-fixes`.

Not all rules have an automatic fix.
For example, the rule `for_loop_index` reports cases such as `for (x in foo(x))`, which is problematic because `x` is both in the index and in the sequence component of the loop.
It is recommended to rename `x` to disambiguate its use, but this requires manual intervention.

::: {.callout-warning}
### Automatic fixes and version control

Using `--fix` may modify several files at once depending on the path you specified.
It can be hard to inspect the changes or to revert a large number of changes, so Jarl provides two safeguards:

1. if the file isn't tracked by a Version Control System (VCS, such as Git), then fixes are not applied and you need to specify `--allow-no-vcs` to apply them;
2. if the file is tracked by a VCS but the status isn't clean (meaning that some files aren't committed), then fixes are not applied and you need to specify `--allow-dirty` to apply them. This is to prevent cases where fixes would be mixed together with other unrelated changes and therefore hard to inspect.
:::

Note that Jarl is not a code formatter, so automatic fixes may not match your expected code style.


## Configuration

Jarl can be configured to select or ignore specific rules, include or exclude files and folders, and apply rule-specific options, among other things.
Those settings can be stored in a `jarl.toml` file so that everyone contributing in a project use the same configuration.

See the [Configuration file](reference/config-file.md) reference for all options.


## Suppression comments

You can use `# jarl-ignore` comments (aka *suppression comments*) to ignore a diagnostic on a specific piece of code (e.g. for a false positive).
Those follow a very specific syntax, see [Suppression comments](howto/suppression-comments.md) for the full guide.


## Editor integration

Jarl integrates with VS Code, Positron, Zed, Helix, and Neovim to provide inline diagnostics and quick fixes:

![](img/code_highlight.PNG){fig-alt="R script with `any(is.na(x))` underlined in yellow, indicating a rule violation. A popup shows Jarl message."}

See [Editor setup](howto/editors.md) for installation instructions.


## CI and pre-commit

Jarl can be used in continuous integration (such as GitHub Actions via [`setup-jarl`](https://github.com/etiennebacher/setup-jarl)).
There is also built-in support for pre-commit hooks.

See [Continuous integration](howto/ci.md) and [Pre-commit tools](howto/precommit.md) for examples.
