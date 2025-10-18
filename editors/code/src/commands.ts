import * as vscode from "vscode";

import { Ctx } from "./context";

export function registerCommands(ctx: Ctx) {
	ctx.extension.subscriptions.push(
		vscode.commands.registerCommand(
			"jarl.restart",
			async () => await ctx.lsp.restart(),
		),
	);
}
