import * as vscode from "vscode";

import { Cmd, Ctx } from "../context";
import {
	saveAllDirtyWorkspaceTextDocuments,
	selectWorkspaceFolder,
} from "../workspace";
import { workspaceFolderFormatting } from "./workspace-folder-formatting";
import { fileExists } from "../utils";

/**
 * Initialize a workspace folder for use with Air
 *
 * - Creates an empty `air.toml` if neither `air.toml` nor `.air.toml` already exist
 * - Creates `extensions.json` recommending `Posit.air-vscode`
 * - Updates `settings.json` to format-on-save R and Quarto, and sets default formatters
 * - Updates `.Rbuildignore` to ignore `.vscode/` and `air.toml` if one existed
 *
 * Optionally, the user can request an immediate formatting of the workspace folder
 * after initialization
 */
export async function workspaceFolderInitialization(
	workspaceFolder: vscode.WorkspaceFolder,
	binaryPath: string,
) {
	const allSaved = await saveAllDirtyWorkspaceTextDocuments(workspaceFolder);
	if (!allSaved) {
		return;
	}

	await createAirToml(workspaceFolder);
	await createExtensionsJson(workspaceFolder);
	await updateSettingsJson(workspaceFolder);
	await updateRbuildignore(workspaceFolder);

	const message = `Successfully initialized the ${workspaceFolder.name} workspace folder.`;
	const shouldFormatItem = `Click to format the ${workspaceFolder.name} workspace folder`;

	const item = await vscode.window.showInformationMessage(
		message,
		shouldFormatItem,
	);

	if (item === shouldFormatItem) {
		await workspaceFolderFormatting(workspaceFolder, binaryPath);
	}
}

export function workspaceFolderInitializationCallback(ctx: Ctx): Cmd {
	return async () => {
		const workspaceFolder = await selectWorkspaceFolder();

		if (!workspaceFolder) {
			return;
		}

		const binaryPath = ctx.lsp.getBinaryPath();

		await workspaceFolderInitialization(workspaceFolder, binaryPath);
	};
}

async function createAirToml(
	workspaceFolder: vscode.WorkspaceFolder,
): Promise<void> {
	const airTomlUri = vscode.Uri.joinPath(workspaceFolder.uri, "air.toml");
	const dotAirTomlUri = vscode.Uri.joinPath(workspaceFolder.uri, ".air.toml");

	if ((await fileExists(airTomlUri)) || (await fileExists(dotAirTomlUri))) {
		return;
	}

	const content = new TextEncoder().encode("");
	await vscode.workspace.fs.writeFile(airTomlUri, content);
}

/**
 * Create `extensions.json` with Air recommended
 *
 * Unlike `settings.json`, VS Code has no API for `extensions.json`, so an
 * existing one is left untouched rather than parsed and rewritten.
 */
async function createExtensionsJson(
	workspaceFolder: vscode.WorkspaceFolder,
): Promise<void> {
	const vscodeUri = vscode.Uri.joinPath(workspaceFolder.uri, ".vscode");
	const extensionsUri = vscode.Uri.joinPath(vscodeUri, "extensions.json");

	if (await fileExists(extensionsUri)) {
		return;
	}

	if (!(await fileExists(vscodeUri))) {
		await vscode.workspace.fs.createDirectory(vscodeUri);
	}

	let contents = `
{
    "recommendations": [
        "Posit.air-vscode"
    ]
}
	`;
	contents = contents.trim();
	contents = contents + "\n";

	const bytes = new TextEncoder().encode(contents);

	await vscode.workspace.fs.writeFile(extensionsUri, bytes);
}

/**
 * VS Code can't update a single key of a language section (e.g.
 * `[r].editor.formatOnSave`). A plain `update("[r]", value)` wipes the other
 * keys and `get()` pulls in inherited global values, so we merge into the
 * folder-scoped value from `inspect()`. Comments in these sections are lost.
 */
async function updateSettingsJson(
	workspaceFolder: vscode.WorkspaceFolder,
): Promise<void> {
	const config = vscode.workspace.getConfiguration(
		undefined,
		workspaceFolder,
	);

	const configR = config.inspect("[r]")?.workspaceFolderValue || {};
	await config.update(
		"[r]",
		{
			...configR,
			"editor.formatOnSave": true,
			"editor.defaultFormatter": "Posit.air-vscode",
		},
		vscode.ConfigurationTarget.WorkspaceFolder,
	);

	const configQuarto = config.inspect("[quarto]")?.workspaceFolderValue || {};
	await config.update(
		"[quarto]",
		{
			...configQuarto,
			"editor.formatOnSave": true,
			"editor.defaultFormatter": "quarto.quarto",
		},
		vscode.ConfigurationTarget.WorkspaceFolder,
	);
}

async function updateRbuildignore(
	workspaceFolder: vscode.WorkspaceFolder,
): Promise<void> {
	const rbuildignoreUri = vscode.Uri.joinPath(
		workspaceFolder.uri,
		".Rbuildignore",
	);

	if (!(await fileExists(rbuildignoreUri))) {
		// Not an R package (or a rare one without `.Rbuildignore`)
		return;
	}

	const buffer = await vscode.workspace.fs.readFile(rbuildignoreUri);
	const content = new TextDecoder().decode(buffer);

	const newline = content.includes("\r\n") ? "\r\n" : "\n";
	const lines = content.split(newline);

	if (lines.at(-1) === "") {
		// Trailing newline: avoid a blank line before the appended patterns
		lines.pop();
	}

	let anyMissing = false;
	const patterns = ["^\\.vscode$", "^[.]?air[.]toml$"];

	for (const pattern of patterns) {
		const exists = lines.find((line) => line === pattern);

		if (!exists) {
			anyMissing = true;
			lines.push(pattern);
		}
	}

	if (anyMissing) {
		const content = lines.join(newline) + newline;
		const buffer = new TextEncoder().encode(content);
		await vscode.workspace.fs.writeFile(rbuildignoreUri, buffer);
	}
}
