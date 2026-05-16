# empty_file

::: {.callout-note title="Added in 0.5.\*" .low-opacity}
:::

## What it does

Reports R files that contain no code: either truly empty, only whitespace, or only comments.

## Why is this bad?

An empty or comment-only file is almost always a mistake: a placeholder that was forgotten, an accidental `touch`, or leftover from a refactor. It adds noise to the package and can confuse readers.

## Example

```r
# TODO: implement the data loader
```

Use instead: delete the file, or add the intended code.
