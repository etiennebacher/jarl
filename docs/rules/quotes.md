# quotes
## What it does

Checks for consistency of quote delimiters in string literals.
This rule is disabled by default.

## Why is this bad?

Using a consistent quote delimiter improves readability.

By default, this rule expects double quotes (`"`). To prefer single quotes,
set this in `jarl.toml`:

```toml
[lint.quotes]
quote = "single"
```

For regular strings, this rule allows the opposite quote when needed to
avoid escaping the preferred quote.

Raw strings follow the same rule and allow the use of the opposite quote
for readability and to prevent early termination.

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
