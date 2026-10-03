# todo_comment

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Reports comments that start with `TODO` or `FIXME`, ignoring case. The marker
must follow the opening `#` characters, an optional roxygen `'`, and optional
whitespace.

A marker is reported when followed by the end of the comment, whitespace,
a symbol, or a digit. It is ignored when immediately followed by a Unicode
letter. For example, `# TODO123` and `# TODO: fix this` are reported, while
`# TODOLIST` and `# TODO中文` are ignored. Strings and markers in the middle
of comment text are ignored.

## Why is this bad?

These comments can indicate unfinished work that should be reviewed before
releasing code.

This rule is disabled by default and has no automatic fix.

## Example

```r
# TODO: handle missing values
x <- 1 # FIXME
```

Complete the work described by the comments, then remove the markers.
