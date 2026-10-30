# one_call_pipe

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Reports pipes whose expression contains only one function call. This applies
to both the native pipe (`|>`) and magrittr pipes (`%>%`, `%!>%`, `%T>%`,
and `%<>%`). This rule is disabled by default.

Calls anywhere in the piped expression count, including calls on the left
side. An expression on the left side without a call, such as `x + 1`, does
not add to the count; an expression containing a call, such as `f(x) + 1`,
does. Pipes whose right-hand call already has arguments, such as
`df |> select(x)`, are not reported.

## Why is this bad?

A pipe with only one call is often clearer as a regular function call.

A safe automatic fix is available for `|>` and `%>%` when the right-hand
side is a call with no arguments and the pipe expression contains no
comments. Other magrittr pipe variants have different semantics and are
reported without a fix.

## Example

```r
x |> sum()
1:3 %>% mean()
```

Use instead:

```r
sum(x)
mean(1:3)
```

A pipe with multiple calls is not reported, for example:

```r
rowSums(x) %>% mean()
```
