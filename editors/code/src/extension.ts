import * as vscode from "vscode";

import { registerCommands } from "./commands";
import { Ctx } from "./context";
import { Lsp } from "./lsp";

let ctx: Ctx;

export function activate(context: vscode.ExtensionContext) {
	let lsp = new Lsp(context);
	lsp.start();

	ctx = new Ctx(context, lsp);
	registerCommands(ctx);

	return {
		// API for our own tests. Tests run outside the extension host, so
		// importing `src` files directly would create a second, disconnected
		// instance of the extension.
		__private: {
			ctx: ctx,
		},
	};
}

export function deactivate() {
	ctx.lsp.stop();
}
