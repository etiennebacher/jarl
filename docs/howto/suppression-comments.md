---
title: Suppression comments
---

It is sometimes needed to ignore diagnostics on certain lines of code, for instance in case of (hopefully rare) false positives.
Jarl supports this via `# jarl-ignore` comments (aka *suppression comments*, because they are used to suppress diagnostics).
In short, Jarl provides four types of suppression comments:

- standard comments: `# jarl-ignore <rule-name>: <reason>`
- range comments: `# jarl-ignore-start <rule-name>: <reason>` and `# jarl-ignore-end <rule-name>`
- file comments: `# jarl-ignore-file <rule-name>: <reason>`
- chunk comments:
  ```
  #| jarl-ignore-chunk:
  #|   - <rule-name>: <reason>
  ```

These comments must follow several rules regarding their syntax and their placement.


## How should I write suppression comments?

All suppression comments follow the same syntax:

1. **A suppression comment must always specify a rule**.
   For instance, `# jarl-ignore any_is_na: <reason>` only suppresses diagnostics of the `any_is_na` rule.
   This means that if you wish to ignore multiple rules for the same code block, you must have one comment per rule to ignore (the reason for this is the next point).
   This also means that comments such as `# jarl-ignore` (aka *blanket suppressions*) are ignored and even reported by Jarl (see the section ["How can I check that my suppression comments are correct?"](#how-can-i-check-that-my-suppression-comments-are-correct) below).

2. **A suppression comment must always specify a reason**.
   Ideally, you shouldn't have to use suppression comments.
   If you don't want any diagnostics of a specific rule to be reported, you should exclude the rule in `jarl.toml` or in the command line arguments.
   Therefore, if you need to use a suppression comment, it means that something went wrong (for instance Jarl reported a false positive).
   In this case, adding an explanation is useful so that other people (or future you) know why this comment is here.
   A reason is any text coming after the colon in `# jarl-ignore any_is_na: <reason>`.



## Where should I place suppression comments?

**Standard comments** apply to an entire *block* of code and not to specific *lines*.
This distinction is important but might seem a bit strange, so let's take an example to clarify:

```r
y <- any(is.na(x))

z <- any(
  is.na(x)
)
```

In this case, the only difference between `y` and `z` is that the former is written on one line and the latter is written over multiple lines.
For Jarl, this doesn't matter: all it sees is the code and not whether it is on multiple lines.
Therefore, adding a suppression comment above `y` and `z` ignores both diagnostics.

```r
# jarl-ignore any_is_na: <reason>
y <- any(is.na(x))

# jarl-ignore any_is_na: <reason>
z <- any(
  is.na(x)
)
```

A slightly more complex example would be this:

```r
f <- function(x1, x2) {
  any(is.na(x1))
  any(is.na(x2))
}

z <- any(is.na(x3))
```

Here, placing a comment above `any(is.na(x1))` would only hide this diagnostic and not `any(is.na(x2))`.
However, placing it above `f <- function(x1, x2) {` would remove both diagnostics in the function definition because both are part of the same block.
Either way, the third diagnostic `z <- any(is.na(x3))` would still be reported because none of the comments apply to this code block.

---

**Range comments** allow you to hide all diagnostics between the start and end comments.
Using again the example above, we could hide all diagnostics with range comments:

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
For example, in the code below, no diagnostic would be reported.

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

---

**Chunk comments** are only valid for R Markdown and Quarto files.
They apply to the entire chunk in which they are located (note that the other suppression comments also work in R code chunks).
Because of the way Quarto works, `#| jarl-ignore-chunk` must be an array.
For example, this doesn't work:

```
#| jarl-ignore-chunk any_is_na: <reason>
```

But this does:

```
#| jarl-ignore-chunk:
#|   - any_is_na: <reason>
```
Because an array is required, one can list several rules and their reasons:

```
#| jarl-ignore-chunk:
#|   - any_is_na: <reason>
#|   - any_duplicated: <another reason>
```

## How can I check that my suppression comments are correct?

By default, Jarl comes with several checks for suppression comments.
Those are not different from other rules so they can be deactivated, but it is recommended not to do so because **wrong comments will be silently ignored by Jarl**.
The checks on suppression comments are listed below:

- `blanket_suppression`: reports comments that don't specify a rule, e.g., `# jarl-ignore: <reason>`.

- `invalid_chunk_suppression`: reports chunk comments that are wrongly formatted (available in R Markdown and Quarto files only), e.g., `#| jarl-ignore-chunk any_is_na: <reason>`.

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


## How can I automatically add suppression comments?

There are two ways to automatically add suppression comments: via the editor integration and via the command line.

If you use a [supported editor](editors.md), clicking on violations will give you several choices, including inserting a suppression comment for the given violation.
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

<!-- Yes, it is "parsimony" and not "parcimony". -->

Note that automatically inserting comments should be used with parsimony and not to hide all diagnostics from the start.
Use a custom configuration to entirely ignore rules you don't want to use, and use `--fix` to automatically fix a certain number of diagnostics.

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


## Compatibility with `lintr`

Jarl's suppression comment system is quite different from [`lintr`'s](https://lintr.r-lib.org/articles/lintr.html#exclusions) as they require a different syntax, different locations relative to the violating code, and have different capabilities.

The good news is that the syntax is so different that one can safely use `lintr` and Jarl in the same project and be sure that their suppression comments will not conflict.
If you wish to transition `lintr` comments to Jarl, the best way to do so is probably to do a global "search and replace" to remove `# nolint` comments, and then use `--add-jarl-ignore` in the command line to add Jarl's comments.
