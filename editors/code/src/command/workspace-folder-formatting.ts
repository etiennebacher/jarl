import * as vscode from "vscode";

import { Cmd, Ctx } from "../context";
import * as output from "../output";
import { isError, isResult, runCommand } from "../process";
import {
	saveAllDirtyWorkspaceTextDocuments,
	selectWorkspaceFolder,
} from "../workspace";

/**
 * Format a workspace folder with the CLI
 *
 * Dirty tabs are saved first because the CLI only sees what is on disk. They
 * stay open: VS Code keeps the LSP client and server in sync with on-disk
 * changes as long as files were saved beforehand.
 *
 * The CLI ignores document settings forwarded by the client, so without an
 * `air.toml` the default indentation applies. Requiring one at the root isn't
 * an option since it may legitimately live deeper, e.g. `{root}/R/air.toml`.
 */
export async function workspaceFolderFormatting(
	workspaceFolder: vscode.WorkspaceFolder,
	binaryPath: string,
) {
	const allSaved = await saveAllDirtyWorkspaceTextDocuments(workspaceFolder);
	if (!allSaved) {
		return;
	}

	const workspaceFolderPath = workspaceFolder.uri.fsPath;

	const args = ["format", workspaceFolderPath, "--no-color"];

	// Redundant with the explicit path, but keeps relative lookups predictable
	const options = {
		cwd: workspaceFolderPath,
	};

	const result = await runCommand(binaryPath, args, options);

	let anyErrors = false;

	if (isError(result)) {
		// Something went horribly wrong in the process spawning or shutdown process
		anyErrors = true;
		output.log(
			`Errors occurred while formatting the ${workspaceFolder.name} workspace folder.\n${result.error.message}`,
		);
	}

	if (isResult(result)) {
		if (result.code !== 0) {
			// Air was able to run and exit, but we had an error along the way
			output.log(
				`Errors occurred while formatting the ${workspaceFolder.name} workspace folder.\n${result.stderr}`,
			);
			anyErrors = true;
		}
	}

	if (anyErrors) {
		const answer = await vscode.window.showInformationMessage(
			`Errors occurred while formatting the ${workspaceFolder.name} workspace folder. View the logs?`,
			{ modal: true },
			"Yes",
			"No",
		);

		if (answer === "Yes") {
			output.show();
		}

		return;
	}

	vscode.window.showInformationMessage(
		`Successfully formatted the ${workspaceFolder.name} workspace folder.`,
	);
}

export function workspaceFolderFormattingCallback(ctx: Ctx): Cmd {
	return async () => {
		const workspaceFolder = await selectWorkspaceFolder();

		if (!workspaceFolder) {
			return;
		}

		const binaryPath = ctx.lsp.getBinaryPath();

		await workspaceFolderFormatting(workspaceFolder, binaryPath);
	};
}
