---
title: Integrations
---

## Github Actions

Jarl can be used as a Github Action via [`setup-jarl`](https://github.com/etiennebacher/setup-jarl).
This action runs in a couple of seconds and will fail if there are any rule violations.
It is possible to customize the arguments passed to Jarl in this action, such as the input paths.

Here is an example YAML file:

```yml
on:
  push:
    branches: main
  pull_request:

name: jarl-check

permissions: read-all

jobs:
  jarl-check:
    name: jarl-check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: etiennebacher/setup-jarl@v0.1.0
```

See the `setup-jarl` repository for more examples.
