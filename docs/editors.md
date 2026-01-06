---
title: Editor support
---

::: {.callout-note}
Version numbers of the Jarl extensions may differ from the version number of Jarl itself.
This is made on purpose so that it is easier to make releases that are specific to each extension or to Jarl itself.

To get the version number of Jarl itself, use `jarl --version`.
:::

## VS Code / Positron

Both VS Code and Positron have access to the Jarl extension via the [VS Marketplace](https://marketplace.visualstudio.com/items?itemName=EtienneBacher.jarl-vscode) and [Open VSX](https://open-vsx.org/extension/etiennebacher/jarl-vscode).

This extension provides code higlights and quick fixes:

* code highlights will underline pieces of code that violate any rule in your setup:

![](img/code_highlight.PNG){fig-alt="R script with `any(is.na(x))` underlined in yellow, indicating a rule violation. A popup shows Jarl message."}

* quick fixes lightbulb icons will appear when the cursor is next to a highlighted piece of code. Clicking this icon will give you several options: apply the fix only for this piece of code, add a comment to ignore this specific violation, or add a comment to ignore all violations present in this piece of code. The screenshots below show the procedure to apply the fix:

![](img/code_quick_fix_1.PNG){fig-alt="R script showing the code `any(is.na(x))`. A blue lightbulb shows that a quick fix is available for this piece of code."}

![](img/code_quick_fix_2.PNG){fig-alt="After clicking on the lightbulb, a popup appears with a button to automatically apply the fix."}

![](img/code_quick_fix_3.PNG){fig-alt="The fix has been applied, the screenshot now shows `anyNA(x)`."}

Use [`jarl.toml`](config.md) to configure Jarl (rules to select or ignore, files to skip, assignment operator to use, etc.).

::: {.callout-tip}
The [Tombi extension](https://github.com/tombi-toml/tombi) is useful to have suggestions and autocompletion when editing `jarl.toml`.
:::


## Zed

Jarl is available in the list of Zed extensions.
After installing it, you will need to update `settings.json`, in particular the field `languages`:

```json
"languages": {
  "Python": {
    [...]
  },
  "R": {
    "language_servers": ["jarl"]
  }
}
```

`language_servers` accepts multiple values, so you may have `"language_servers": ["jarl", "air"]` for example.

## RStudio

Currently, Jarl cannot be integrated in RStudio to highlight code or provide quick fix actions in the editor.
The only way to use Jarl in RStudio is via the Terminal.

## Helix

To use Jarl as language server in the Helix editor, you need first to add it to the [language configuration file](https://docs.helix-editor.com/languages.html), for instance `~/.config/helix/languages.toml`:

```toml
[language-server.jarl]
command = "jarl"
args = ["server"]

[[language]]
name = "r"
language-servers = ["jarl"]
```

Jarl should then be active in the editor, providing code highlighting and showing the message when the cursor is on the highlighted code:

![](img/helix_highlight.png){fig-alt="R script showing several pieces of code that trigger rule violations, such as `any(is.na(x))`. This is displayed in the Helix editor. The code is underlined in yellow and the violation message appears next to the code."}

Helix also provides a code-action keybinding.
When the cursor is on some code reported by Jarl and when the editor is in "Normal" mode, press "Space" then "a" to show the different code actions:

![](img/helix_quick_fix.png){fig-alt="The same R script as before, but this time there is a list of three actions next to the highlighted piece of code: apply fix, ignore this rule, and ignore all rules."}

## Neovim

To use Jarl as an LSP in Neovim, you need to configure it with the [built-in Neovim LSP (vim.lsp.config)](https://neovim.io/doc/user/lsp.html#lsp-config) or through nvim-lspconfig. It is not yet available through [Mason](https://github.com/williamboman/mason.nvim) or part of the nvim-lspconfig collection. Below is an example using the built-in system with Neovim 0.11+.

Create an LSP config file at `~/.config/nvim/lsp/jarl.lua`:
```lua
---@type vim.lsp.Config
return {
  cmd = { 'jarl', 'server' },
  filetypes = { 'r', 'rmd'},
  -- root_markers = { '.git' },
  root_dir = function(bufnr, on_dir)
    on_dir(vim.fs.root(bufnr, '.git') or vim.uv.os_homedir())
  end,
}
```

Then enable the server in your Neovim configuration (e.g. `init.lua` or `lspconfig.lua`):

```lua
-- simply
vim.lsp.enable 'jarl'
-- or to pass custom config command
vim.lsp.config('jarl', {})
vim.lsp.enable 'jarl'
```

This enables the code-actions and diagnostics.

![](img/nvim_diagnostic.png){fig-alt="R script with multiple errors showing in-line indicating a rule violation."}

![](img/nvim_quick_fix.png){fig-alt="The same R script as before, but this time there is a list of three actions next to the piece of code: apply fix, ignore this rule, and ignore all rules."}
