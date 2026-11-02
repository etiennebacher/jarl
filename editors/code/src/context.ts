import * as vscode from "vscode";
import { Lsp } from "./lsp";

// Parts of this file are adapted from
// https://github.com/rust-lang/rust-analyzer/blob/master/editors/code/src/ctx.ts

export class Ctx {
	constructor(
		readonly extension: vscode.ExtensionContext,
		public lsp: Lsp,
	) {}
}
