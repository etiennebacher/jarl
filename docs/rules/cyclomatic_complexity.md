# cyclomatic_complexity

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Measures the cyclomatic complexity of each function and reports the ones above
`max-complexity` (20 by default).

When a function is above the maximum, the message also lists the expressions
in its body that add more than 20% of the score, as a starting point to
split it.

## Why is this bad?

The score counts the paths through the flow graph of the function.
A function with a high score can be hard to read, to test exhaustively, and
to change without breaking one of the paths nobody had in mind.

A high score might be due to a large number of `if` conditions, `for` loops,
or early `return()` calls, among other things. It generally indicates that
the function should be refactored, for instance by creating smaller
intermediate functions.

## Details

While the way the score is computed tries to match the output of the
[`cyclocomp`](https://cran.r-project.org/package=cyclocomp) package (which is
also the output of `lintr::cyclocomp_linter()`), some differences exist:

* a chain of `&&` or `||` adds one path per operator. `cyclocomp` scores
  `if (a || b || c || d) stop()` as 11, this rule as 5 (`stop()` can be
  reached via paths, and we add one path if the condition is never true);
* `switch()` adds one path per arm, while `cyclocomp` ignores it;
* a function defined inside another one is scored on its own and counts as
  a single node in the function that contains it, while `cyclocomp` adds all
  of its paths to the ones of the outer function.

## Configuration options

The operation of the `cyclomatic_complexity` rule can be customised in the [configuration file](../reference/config-file.md).

Use `max-complexity` to set the highest score a function is allowed to
reach before it is reported. It must be at least 1.

### Default values

```toml
max-complexity = 20
```

### TOML settings

```toml
[lint]
...

[lint.cyclomatic_complexity]
max-complexity = 25
```

## References

* [Cyclomatic complexity](https://en.wikipedia.org/wiki/Cyclomatic_complexity)
