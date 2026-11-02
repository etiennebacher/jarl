# download_file

::: {.callout-note title="Added in 0.0.24" .low-opacity}
:::

## What it does

Checks for missing or non-binary `mode` arguments in `download.file()`, and
for `mode` arguments ignored by `method = "curl"` or `method = "wget"`.

## Why is this bad?

Text modes `"w"` and `"a"` can corrupt binary downloads on Windows. Use
`"wb"` or `"ab"` instead. With the external `curl` and `wget` methods,
`mode` is ignored and should be omitted.

When `mode` is missing, literal URLs ending in `.gz`, `.bz2`, `.xz`, `.tgz`,
`.zip`, `.jar`, `.rda`, `.rds`, or `.RData` are exempt because R automatically
uses binary mode for them on Windows. The check is case-sensitive and uses
the URL after percent-decoding, not the destination filename. Explicit text
modes are still reported for these URLs.

## Example

```r
download.file(url, destfile)
download.file(url, destfile, mode = "a")
download.file(url, destfile, method = "curl", mode = "wb")
```

Use instead:
```r
download.file(url, destfile, mode = "wb")
download.file(url, destfile, mode = "ab")
download.file(url, destfile, method = "curl")
```

## References

See `?download.file`
