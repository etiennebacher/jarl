# quotes
::: {.callout-note title="Added in 0.5.0" .low-opacity}
:::

## What it does

Checks for consistency of quote delimiters in string literals.
This rule is disabled by default.

## Why is this bad?

In general, single (`'`) and double (`"`) quotes can be used
interchangeably within R.
However, inconsistent use of quote styles decreases readability.

Base R documentation and the Tidyverse style guide recommend using double
quotes for all strings, except for when the string already contains double
quotes. Therefore, by default, this rule expects double quotes (`"`).

To prefer single quotes, set this in `jarl.toml`:
```toml
[lint.quotes]
quote = "single"
```

For regular strings, this rule allows the opposite quote when needed to
avoid escaping the preferred quote. For example,
```r
cat('R says "Hello world" ...')
```
is easier to read than
```r
cat("R says \"Hello world\" ...")
```

Raw strings also allow the use of the opposite quote for readability and to
prevent early termination.

For example:
```r
r'("rawstring")'
```
is more readable than
```r
r"("rawstring")"
```

Using the wrong delimiter can also terminate the string early. For example:
```r
r'(abc)"def)'
```
is valid R, but
```r
r"(abc)"def)"
```
results in early termination and a syntax error.

## Example

```r
x <- 'hello'
print(r'-('hello')-')
```

Use instead:
```r
x <- "hello"
print(r"-('hello')-")
```

## References

See:

- [Tidyverse style guide](https://style.tidyverse.org/syntax.html#character-vectors)
- [R documentation](https://stat.ethz.ch/R-manual/R-patched/library/base/html/Quotes.html)
