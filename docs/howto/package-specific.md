---
title: Package-specific rules
---

As of 0.5.0, Jarl allows package-specific rules, such as rules that only apply to `dplyr` functions.
Those rules are more complicated to handle than the others, so this page will present some of their differences and limitations, and then explain why that is.

## Differences in usage and limitations

### No multi-file analysis

To know which packages are present in the namespace of a given R script, Jarl looks at `library()` and `require()` calls only in the same script.
For instance, if you have `analysis.R` and `master.R` like this:

* `master.R`:

  ```r
  library(dplyr)
  ```

* `analysis.R`:

  ```r
  source("master.R")
  filter()
  ```

then Jarl will not consider that `filter()` can come from `dplyr`.

The exception for multi-file analysis is for R packages: if `dplyr` is imported (either fully or partially), then all files in the `R` folder have access to it.

### R must be installed

If you want to use package-specific rules (all disabled by default), you must have R installed.
This might seem an obvious requirement, but it makes a difference if you want to use Jarl in CI since you will now need to install R and the packages in your project before running Jarl.

### Your system matters

This was already a bit true because some rules depend on your R version, but it's even more important for  package-specific rules.
Some of those rules are only valid for some package versions, so depending on your system, some diagnostics may appear or disappear.
Jarl checks packages stored in `.libPaths()`, meaning that if you use `renv` for instance, it will grab the versions of packages stored in `renv/library`.

If the project that is checked is an R package, Jarl will look at the content of `DESCRIPTION` and `NAMESPACE`.


## Why those differences?

This section explains broadly how Jarl handles package-specific rules internally, it is not required to know that to use Jarl (all the necessary information is above).

### Static analysis vs dynamic namespace

To implement package-specific rules, we must be able to tell from which package a function comes from.
This is relatively simple when we have access to an R session:

```r
filter
#> function (x, filter, method = c("convolution", "recursive"),
#>     sides = 2L, circular = FALSE, init = NULL)
#> {
#>   [...]
#> }
#> <bytecode: 0x5b3495e0cfe0>
#> <environment: namespace:stats>

library(dplyr, warn.conflicts = FALSE)

filter
#> function (.data, ..., .by = NULL, .preserve = FALSE)
#> {
#>     [...]
#> }
#> <bytecode: 0x630942de7a38>
#> <environment: namespace:dplyr>
```

The `namespace:` shows the origin of the function.
However, Jarl does static analysis and therefore doesn't run R code, in part because it would slow it down.

Then, the question is: how can we resolve a function's origin without running R code, or with running minimal R code?

This is quite easy in other languages, such as Python:

```python
import pandas as pd
import polars as pl

pl.DataFrame() # <---- we know this comes from polars
```

Using explicit namespaces such as `dplyr::filter()` would make this task straightforward, but it is very common to load packages once with `library()` and then rely on the implicit namespace resolution, making this way more challenging.
To solve this, Jarl has to run a very short R script (but only in the case where some package-specific rules are enabled).


### Getting package namespaces and versions

Once we have collected the list of packages used in a script from `library()` and `require()` calls, we need to get their versions and their namespaces.
This is where we need to run a small R script calling `packageVersion()` and `getNamespaceExports()` for the packages we're interested in.


### Using this information in rules

Now that we have collected the packages whose namespaces are available in a given script and the version of these packages, we can use this information in the rule.
We try to resolve the origin of a function, which can lead to several situations:

- the origin is resolved, meaning that the function comes from a single package:
  - if the package is what we want, keep going;
  - otherwise, exit the rule.

- the origin is unknown, meaning that we don't know where the function comes from. This can happen when the `library()` calls is located in another script. In this case, we exit the rule.

- the origin is ambiguous, meaning that the function is provided by two or more packages. In this case, whether we should exit the rule or not depends on the context. For instance, if we're adding a rule for `dplyr::filter()`, we might want to assume that a `filter()` in a chain of pipes belongs to `dplyr` but a `filter()` outside a chain doesn't.


::: {.callout-note title="Why not using packages loading order?"}
The order in which packages are loaded is very important because it tells us which namespace is used in case of namespace conflict.

We might think we can use this information to resolve the origin of a function, but this is not necessarily the case because there exists ways to explicitly prefer a certain package after it is loaded, such as the [`conflicted`](https://conflicted.r-lib.org/index.html) package.
:::

