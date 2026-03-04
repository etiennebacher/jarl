---
title: Dealing with R versions
---

Some rules depend on the R version that is used in the project.
For example, `grepv` recommends the use of `grepv()` over `grep(value = TRUE)`, but this rule only makes sense if the project uses `R >= 4.5.0` since this function was introduced in this version.

By default, when the R version used in the project cannot be retrieved, Jarl doesn't apply rules that depend on an R version.
There are two ways to tell Jarl which R version you're using:

1. you can pass this information by hand using `--min-r-version`. For example, passing `--min-r-version 4.3` will tell Jarl that it can apply rules that depend on R 4.3.0 or before. Rules that depend on R 4.3.1 or more would still be ignored.
1. if your project has a `DESCRIPTION` file, you can set `R (>= x.y.z)` in the `Depends` field and Jarl will retrieve this version.
