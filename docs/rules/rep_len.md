# rep_len
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for usage of `rep(x, length.out = n)`.

## Why is this bad?

`rep(x, length.out = n)` calls `rep_len(x, n)` internally. The latter
is thus more direct and equally readable.

This rule is disabled by default.

This rule has an unsafe automatic fix because `rep_len()` drops most
attributes, including names, while `rep()` can preserve them.

## Example

```r
rep(1:3, length.out = 10)
```

Use instead:
```r
rep_len(1:3, 10)
```

## References

See `?rep`
