---
title: Using Jarl
---

## Linting and fixing

`jarl check` is the command required to diagnoze one or several files.
It takes a path as its first argument, such as `jarl check .` to check all files starting from the current directory.
This command will return a list of diagnostics, one per rule violation.

This is already useful information, but it can be tedious to fix those violations one by one.
To help addressing this issue, Jarl can apply automatic fixes to some of those diagnostics.
This is done simply by passing the argument `--fix`, such as `jarl check . --fix`.

For some rules, an automatic fix cannot be inferred simply based on static code analysis.
For example, the rule `for_loop_index` reports cases such as `for (x in foo(x))`, which is problematic because `x` is both in the index and in the sequence component of the loop.
It is recommended to rename `x` to disambiguate its use, but this requires manual intervention.

::: {.callout-warning}
## Automatic fixes and version control

Using `--fix` may modify several files at once depending on the path you specified.
It can be hard to inspect the changes or to revert a large number of changes, so Jarl provides two safeguards:

1. if the file isn't tracked by a Version Control System (VCS, such as Git), then fixes are not applied and you need to specify `--allow-no-vcs` to apply them;
2. if the file is tracked by a VCS but the status isn't clean (meaning that some files aren't committed), then fixes are not applied and you need to specify `--allow-dirty` to apply them. This is to prevent cases where fixes would be mixed together with other unrelated changes and therefore hard to inspect.
:::

Automatic fixes are distinguished between "safe" and "unsafe".

**Safe fixes** do not change the behavior of the code when it runs, but improve its readability or performance, for instance by using more appropriate functions (see [`any_is_na`](rules/any_is_na.md)).

**Unsafe fixes** may change the behavior of the code when it runs.
For example, [`all_equal`](rules/all_equal.md) reports cases such as `!all.equal(x, y)`.
This code is likely a mistake because `all.equal()` returns a character vector and not `FALSE` when `x != y`.
Jarl could fix this to be `!isTRUE(all.equal(x, y))` instead, but this would change the behavior of the code, so it is marked "unsafe".

By default, only safe fixes are applied.
To apply the unsafe fixes, use `--unsafe-fixes`, e.g. `jarl check . --fix --unsafe-fixes`.

## Selecting and ignoring rules

We can apply a subset of rules using the `--select` and `--ignore` parameters:

```sh
jarl check . --select any_is_na,is_numeric,length_levels
jarl check . --ignore any_duplicated,matrix_apply
```

One could also select rules by family, for instance:

```sh
jarl check . --select PERF,READ
```

to select rules related to performance or readability only.
The list of rule families is available in the ["Rules" page](rules.qmd), and those can be used in all places where selecting and ignoring rules is possible.

## Ignoring diagnostics

It is sometimes needed to ignore diagnostics on certain lines of code, either for all rules or just a subset.
Jarl follows the existing infrastructure of `lintr` and uses special comments (usually called **suppression comments**) starting with `# nolint`.
The node that follows this special comment will be ignored.

Several suppression comments are supported:

1. `# nolint` ignores all rules on the next node;
1. `# nolint: any_duplicated, any_is_na` ignores these two rules only on the next node;
1. `# nolint start` and `# nolint end` mark the start and end of the chunk of code where diagnostics are ignored. It is also possible to use `# nolint start: any_duplicated, any_is_na` to ignore only those two rules on this chunk of code.

::: {.callout-important}
## The case against suppression comments

It can be tempting to put a suppression comment on a piece of code when one doesn't want to bother with fixing the diagnostic.
However, as the project evolves, there is a risk that no one remembers why those comments exist in the first place and the code may now have many pieces of code that could be more readable, efficient, or robust, but are invisible to the linter.

I recommend against using suppression comments, but if you do, you should follow those two points:

1. use targeted comments, e.g. `# nolint: any_is_na`, instead of "blanket" comments like `# nolint` that would hide all diagnostics;
2. add an extra comment explaining why this diagnostic must be ignored.
:::


Note that I have mentioned several times the term "node".
Suppression comments are associated to a node in the representation of R code, and not to a line of code.
In other words, when Jarl inspects comments to determine whether some code should be ignored, it considers these two pieces of code to be equivalent:

```r
# nolint
any(is.na(x))
```

```r
# nolint
any(
  is.na(x)
)
```

What matters is the position of the comment relative to the start of the node.
The following code is different from the two examples above because the comment is now attached to `is.na(x)`, not to the `any()` call anymore:

```r
any(
  # nolint
  is.na(x)
)
```
so this would be reported.

::: {.callout-note}
## Compatibility with `lintr`

To make it easier to switch between `lintr` and Jarl (or to use both), putting the special comment at the end of the node is also supported. For example, this wouldn't be reported:

```r
any(is.na(x)) # nolint
```

Also for compatibilty, rule names that end with "_linter" are supported, so these two are equivalent:

```r
any(is.na(x)) # nolint: any_is_na
any(is.na(x)) # nolint: any_is_na_linter
```
:::

It is also possible to ignore entire files.
Jarl will ignore all files that contain "Generated by" in a comment at the top of the file, to avoid linting autogenerated files (by Roxygen2 or Rcpp for example).
You can also set a list of files to exclude using [`jarl.toml`](config.md#exclude).

## Dealing with R versions

Some rules depend on the R version that is used in the project.
For example, `grepv` recommends the use of `grepv()` over `grep(value = TRUE)`, but this rule only makes sense if the project uses `R >= 4.5.0` since this function was introduced in this version.

By default, when the R version used in the project cannot be retrieved, Jarl doesn't apply rules that depend on an R version.
There are two ways to tell Jarl which R version you're using:

1. you can pass this information by hand using `--min-r-version`. For example, passing `--min-r-version 4.3` will tell Jarl that it can apply rules that depend on R 4.3.0 or before. Rules that depend on R 4.3.1 or more would still be ignored.
1. if your project has a `DESCRIPTION` file, you can set `R (>= x.y.z)` in the `Depends` field and Jarl will retrieve this version.

## Using a configuration file

It is possible to save settings in a `jarl.toml` file. See the [Configuration page](config.md).


::: {.callout-note collapse="true"}
## About colored output

By default, Jarl will print colored output in the terminal.
To deactivate this, set the environment variable `NO_COLOR` to `1`.
For example, in Bash, the following command would return non-colored output:

```sh
NO_COLOR=1 jarl check .
```
:::
