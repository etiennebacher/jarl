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

## Using a configuration file

It is possible to save settings in a `jarl.toml` file. See the [Configuration page](config.md).

## Ignoring diagnostics

It is sometimes needed to ignore diagnostics on certain lines of code, for instance in case of (hopefully rare) false positives.
Jarl supports this via `# jarl-ignore` comments (aka *suppression comments*, because they are used to suppress violations).
In short, Jarl provides three types of suppression comments:

- standard comments: `# jarl-ignore <rule-name>: <reason>`
- range comments: `# jarl-ignore-start <rule-name>: <reason>` and `# jarl-ignore-end <rule-name>`
- file comments: `# jarl-ignore-file <rule-name>: <reason>`

These comments must follow several rules regarding their syntax and their placement.


### How should I write suppression comments?

All suppression comments follow the same syntax:

1. **A suppression comment must always specify a rule**.
   For instance, `# jarl-ignore any_is_na: <reason>` only suppresses violations of the `any_is_na` rule.
   This means that if you wish to suppress multiple rules for the same code block, you must have one comment per rule to suppress (the reason for this is the next point).
   This also means that comments such as `# jarl-ignore` (aka *blanket suppressions*) are ignored by Jarl (and may be reported, see the section ["How can I check that my suppression comments are correct?"](#how-can-i-check-that-my-suppression-comments-are-correct) below).

2. **A suppression comment must always specify a reason**.
   Ideally, you shouldn't have to use suppression comments.
   If you don't want any violations of a specific rule to be reported, you should exclude the rule in `jarl.toml` or in the command line arguments.
   Therefore, if you need to use a suppression comment, it means that something went wrong (for instance Jarl reports a false positive).
   In this case, adding an explanation is useful so that other people (or future you) know why this comment is here.
   A reason is any text coming after the colon in `# jarl-ignore any_is_na: <reason>`.



### Where should I place suppression comments?

**Standard comments** apply to an entire *block* of code and not to specific *lines*.
This distinction is important but might seem a bit strange, so let's take an example to clarify:

```r
y <- any(is.na(x1))

z <- any(
  is.na(x2)
)
```

In this case, the only difference between `y` and `z` is that the former is written on one line and the latter is written over multiple lines.
For Jarl, this doesn't matter: all it sees is the code and not whether it is on multiple lines.
Therefore, one could add a suppression comment above `y` or `z` and both would be ignored.

```r
# jarl-ignore any_is_na: <reason>
y <- any(is.na(x1))

# jarl-ignore any_is_na: <reason>
z <- any(
  is.na(x2)
)
```

Note that the first comment applies only to the block right after it, so we need one comment per block to ignore.
A slightly more complex example would be this:

```r
f <- function(x1, x2) {
  any(is.na(x1))
  any(is.na(x2))
}

z <- any(is.na(x3))
```

Here, placing a comment above `any(is.na(x1))` would only hide this violation and not `any(is.na(x2))`.
However, placing it above `f <- function(x1, x2) {` would remove both violations in the function definition because both are part of the same block.
Either way, the third violation `z <- any(is.na(x3))` would still be reported because none of the comments apply to this code block.

---

**Range comments** allow you to hide all violations between the start and end comments.
Using again the example above, we could hide all violations with range comments:

```r
# jarl-ignore-start any_is_na: <reason>
f <- function(x, y) {
  any(is.na(x))
  any(is.na(y))
}

z <- any(is.na(x))
# jarl-ignore-end any_is_na
```

As you can see, start comments must have the `<reason>` in the comment, but end comments don't have to.
On top of the syntax rules described above, range comments come with a couple more rules.
In particular, all `jarl-ignore-start` must have a matching `jarl-ignore-end`, and vice-versa.
Additionally, those pairs of comments must be at the same nesting level.
For instance, the example above wouldn't be valid if I had put the end comment inside the function body:

```r
# jarl-ignore-start any_is_na: <reason>
f <- function(x, y) {
  any(is.na(x))
  any(is.na(y))
  # jarl-ignore-end any_is_na
}
```

---

**File comments** apply to the entire file.
Those comments start with `# jarl-ignore-file` and must be placed at the top of the file.
This means that they can come after other comments, but they have to be before any piece of code.
For example, the code below wouldn't report any violation.

```r
# Author: Etienne Bacher
# jarl-ignore-file any_is_na: <reason>
# Date: 2026-02-01

f <- function(x, y) {
  any(is.na(x))
  any(is.na(y))
}

z <- any(is.na(x))
```

### How can I check that my suppression comments are correct?

By default, Jarl comes with several checks for suppression comments.
Those are not different from other rules so they can be deactivated, but it is recommended not to do so because the violating comments will be silently ignored by Jarl.
The checks on suppression comments are listed below:

- `blanket_suppression`: reports comments that don't specify a rule, e.g., `# jarl-ignore: <reason>`.

- `misnamed_suppression`: reports comments where the rule doesn't exist, e.g., `# jarl-ignore unknown_rule: <reason>`.

- `misplaced_file_suppression`: reports comments where a file suppression comment is misplaced, e.g.,
  ```r
  x <- 1 + 1
  # jarl-ignore-file any_is_na: <reason>
  any(is.na(y))
  ```

- `misplaced_suppression`: reports comments where a suppression comment is misplaced, e.g.,
  ```r
  any(is.na(y)) # jarl-ignore any_is_na: <reason>
  ```

- `unexplained_suppression`: reports comments that don't have a `<reason>`, e.g.,
  ```r
  # jarl-ignore any_is_na
  any(is.na(y))
  ```

- `unmatched_range_suppression`: reports range comments whose start (or end) doesn't have a corresponding end (or start), e.g.,
  ```r
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(y))
  ```
  This is also reported when start and end comments are not at the same nesting level.
  ```r
  # jarl-ignore-start any_is_na: <reason>
  f <- function(x) {
    any(is.na(y))
    # jarl-ignore-end any_is_na
  }
  ```

- `unused_suppression`: reports comments that don't suppress any violations, meaning that they can be removed, e.g.,
  ```r
  # jarl-ignore any_is_na: <reason>
  x <- 1 + 1
  ```


### How can I automatically add suppression comments?

There are two ways to automatically add suppression comments: via the editor integration and via the command line.

If you use a [supported editor](./editors.md), clicking on violations will give you several choices, including inserting a suppression comment for the given violation.
See more information on the "Editors" page.

You can also use the command line to insert those suppression comments:

```sh
# This will add `# jarl-ignore <rulename>: <reason>` (where <rulename>
# is replaced by the appropriate rule name).
jarl check . --add-jarl-ignore

# This will add `# jarl-ignore <rulename>: <reason>` (where <rulename>
# is replaced by the appropriate rule name, and <reason> is replaced
# by the text below).
jarl check . --add-jarl-ignore="remove this when bug xyz is fixed"
```

::: {.callout-note collapse="false"}
### About formatting

Automatically inserting comments can degrade the formatting of your code.
For example, the following code:

```r
if (x) {
  1
} else if (any(is.na(y))) {
  2
}
```
would become
```r
if (x) {
  1
} else if (
           # jarl-ignore any_is_na: <reason>
           any(is.na(y))) {
  2
}
```

This can be fixed by a proper code formatter (which Jarl isn't).
It's worth noting that Jarl plays particularly well with [Air](https://posit-dev.github.io/air/) because it uses the same infrastructure.
Using Air on the code above would give:

```r
if (x) {
  1
} else if (
  # jarl-ignore any_is_na: <reason>
  any(is.na(y))
) {
  2
}
```
:::


### Compatibility with `lintr`

Jarl's suppression comment system is quite different from [`lintr`'s](https://lintr.r-lib.org/articles/lintr.html#exclusions) as they require a different syntax, different locations relative to the violating code, and have different capabilities.

The good news is that the syntax is so different that one can safely use `lintr` and Jarl in the same project and be sure that their suppression comments will not conflict.
If you wish to transition `lintr` comments to Jarl, the best way to do so is probably to do a global "search and replace" to remove `# nolint` comments, and then use `--add-jarl-ignore` in the command line to add Jarl's comments.


## Dealing with R versions

Some rules depend on the R version that is used in the project.
For example, `grepv` recommends the use of `grepv()` over `grep(value = TRUE)`, but this rule only makes sense if the project uses `R >= 4.5.0` since this function was introduced in this version.

By default, when the R version used in the project cannot be retrieved, Jarl doesn't apply rules that depend on an R version.
There are two ways to tell Jarl which R version you're using:

1. you can pass this information by hand using `--min-r-version`. For example, passing `--min-r-version 4.3` will tell Jarl that it can apply rules that depend on R 4.3.0 or before. Rules that depend on R 4.3.1 or more would still be ignored.
1. if your project has a `DESCRIPTION` file, you can set `R (>= x.y.z)` in the `Depends` field and Jarl will retrieve this version.


::: {.callout-note collapse="true"}
## About colored output

By default, Jarl will print colored output in the terminal.
To deactivate this, set the environment variable `NO_COLOR` to `1`.
For example, in Bash, the following command would return non-colored output:

```sh
NO_COLOR=1 jarl check .
```
:::
