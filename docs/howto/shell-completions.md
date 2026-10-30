---
title: Shell completions
---

Jarl supports completions for many shells: `bash`, `zsh`, `powershell`, `fish`, and
`elvish`.
Once they are enabled, pressing <kbd>Tab</kbd> completes Jarl commands and their
arguments, as well as the values these arguments accept:

```sh
jarl check --select any<TAB>
# any_duplicated  any_is_na
```

Since `--select`, `--extend-select`, and `--ignore` accept a comma-separated list, rule
names are also completed after a comma:

```sh
jarl check --select any_is_na,cl<TAB>
# any_is_na,class_equals
```

## Setup

Run one of the following to add Jarl's completions to your shell's startup procedure,
then restart the shell.

To temporarily turn completions off, set `COMPLETE=` or `COMPLETE=0`.

### Bash

``` bash
echo 'source <(COMPLETE=bash jarl)' >> ~/.bashrc
```

### Elvish

``` bash
echo 'eval (E:COMPLETE=elvish jarl | slurp)' >> ~/.elvish/rc.elv
```

### Fish

``` bash
echo 'COMPLETE=fish jarl | source' > ~/.config/fish/completions/jarl.fish
```

### Powershell

``` powershell
if (!(Test-Path -Path $PROFILE)) {
  New-Item -ItemType File -Path $PROFILE -Force
}
Add-Content -Path $PROFILE -Value '$env:COMPLETE = "powershell"; jarl | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE'
```

### Zsh

``` bash
echo 'source <(COMPLETE=zsh jarl)' >> ~/.zshrc
```

