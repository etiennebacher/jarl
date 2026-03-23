---
title: Environment variables
---

This section lists all environment variables that can be used in Jarl:

* `NO_COLOR`: set this to any value to remove colors from the output.
  - Example: `NO_COLOR=1`

* `JARL_N_VIOLATIONS_HINT_STAT`: Jarl prints a hint to use `--statistics` if the number of violations is higher than a certain threshold. By default, this threshold is 15. This environment variable overrides this value.
  - Example: `JARL_N_VIOLATIONS_HINT_STAT=25`
