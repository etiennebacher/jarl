---
title: Package-specific rules
---

As of 0.5.0, Jarl allows package-specific rules, such as rules that only apply to `dplyr` functions.
Those rules are more complicated to handle than the others, so this page will present some of their differences and limitations, and then explain why that is.

## Differences in usage

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


