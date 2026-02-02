# misplaced_file_suppression
## What it does

Checks for `# jarl-ignore-file` comments that are not at the top of the file.

## Why is this bad?

File-level suppression comments must appear at the very beginning of the file
(before any code) to be applied. A `# jarl-ignore-file` comment placed
elsewhere in the file is silently ignored by Jarl.

## Example

```r
x <- 1

# The comment below isn't applied because it's not at the top of the file.
# jarl-ignore-file any_is_na: <reason>
any(is.na(x))
```

Use instead:
```r
# jarl-ignore-file any_is_na: <reason>

x <- 1
any(is.na(x))
```
