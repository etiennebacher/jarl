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

* quick fixes lightbulb icons will appear when the cursor is next to a highlighted piece of code. Clicking this icon will allow you to apply the fix only for this piece of code:

![](img/code_quick_fix_1.PNG){fig-alt="R script showing the code `any(is.na(x))`. A blue lightbulb shows that a quick fix is available for this piece of code."}

![](img/code_quick_fix_2.PNG){fig-alt="After clicking on the lightbulb, a popup appears with a button to automatically apply the fix."}

![](img/code_quick_fix_3.PNG){fig-alt="The fix has been applied, the screenshot now shows `anyNA(x)`."}


This extension provides few options integrated in VS Code or Positron.
One of them is "Assignment operator", that indicates which of `"="` or `"<-"` is preferred in the files parsed by Jarl.
This option can be set at the User or Workspace level by looking for "Jarl" in the IDE settings.

It is recommended to use [`jarl.toml`](config.md) if more configuration is needed.

::: {.callout-tip}
The [Tombi extension](https://github.com/tombi-toml/tombi) is useful to have suggestions and autocompletion when editing `jarl.toml`.
:::
