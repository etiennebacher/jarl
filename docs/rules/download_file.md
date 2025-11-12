# download_file
## What it does

Checks for usage of `download.file()` with `mode = "a"` or `mode = "w"`.

## Why is this bad?

`mode = "a"` or `mode = "w"` can generate broken files on Windows.
`download.file()` documentation recommends using `mode = "wb"` and
`mode = "a"` instead. If `method = "curl"` or `method = "wget"`, no mode
should be provided as it will be ignored.

## Example

```r
download.file(x = my_url)
download.file(x = my_url, mode = "w")
```

Use instead:
```r
download.file(x = my_url, mode = "wb")
```

## References

See `?download.file`
