# invalid_chunk_suppression
## What it does

Checks for `jarl-ignore-chunk` comments that use a single-line form
instead of the required Quarto YAML array form.

## Why is this bad?

In Quarto and R Markdown documents, `#|` comments are parsed as YAML chunk
options. The single-line form

```r
#| jarl-ignore-chunk any_is_na: <reason>
```

is not idiomatic YAML and therefore Quarto will not compile. The correct
form is a YAML array:

```r
#| jarl-ignore-chunk:
#|   - any_is_na: <reason>
```

## Example

```r
#| jarl-ignore-chunk any_is_na: <reason>
any(is.na(x))
```

Use instead:

```r
#| jarl-ignore-chunk:
#|   - any_is_na: <reason>
any(is.na(x))
```
