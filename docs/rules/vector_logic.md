# vector_logic
## What it does

Checks for calls to `&` and `|` in the conditions of `if` and `while`
statements.

## Why is this bad?

Using `&` and `|` requires evaluating both sides of the expression, which can
be expensive. In contrast, `&&` and `||` have early exits. For example,
`a && b` will not evaluate `b` if `a` is `FALSE` because we already know that
the output of the entire expression will be `FALSE`, regardless of the value of
`b`. Similarly, `a || b` will not evaluate `b` if `a` is `TRUE`.

This rule only reports cases where the binary expression is the top operation
of the `condition` in an `if` or `while` statement. For example, `if (x & y)`
will be reported but `if (foo(x & y))` will not. The reason for this is
that in those two contexts, the length of `condition` must be equal to 1
(otherwise R would error as of 4.3.0), so using `& / |` or `&& / ||`
is equivalent.

This rule doesn't have an automatic fix.

## Example

```r
if (x & y) 1
if (x | y) 1
```

Use instead:
```r
if (x && y) 1
if (x || y) 1
```

## References

See `?Logic`
