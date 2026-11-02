import path from "path";
import * as vscode from "vscode";
import * as fs from "fs-extra";
import * as output from "./output";

/*
 * Locate the "root" workspace folder
 *
 * Iterates through the open workspace folders, looking for the one with the "shortest"
 * path. If no workspaces are open, returns a mock `WorkspaceFolder` representing the
 * current directory.
 *
 * Adapted from:
 * https://github.com/microsoft/vscode-python-tools-extension-template/blob/8f474ec4ac4e7205ffed9f7f243473bb00bf29c0/src/common/utilities.ts#L38
 */
export async function getRootWorkspaceFolder(): Promise<vscode.WorkspaceFolder> {
	const workspaces: readonly vscode.WorkspaceFolder[] = getWorkspaceFolders();

	if (workspaces.length === 0) {
		return {
			uri: vscode.Uri.file(process.cwd()),
			name: path.basename(process.cwd()),
			index: 0,
		};
	} else if (workspaces.length === 1) {
		return workspaces[0];
	} else {
		let rootWorkspace = workspaces[0];
		let root = undefined;

		// Find first existing workspace path
		for (const w of workspaces) {
			if (await fs.pathExists(w.uri.fsPath)) {
				root = w.uri.fsPath;
				rootWorkspace = w;
				break;
			}
		}

		// Update root workspace if we find a shorter path
		for (const w of workspaces) {
			if (
				root &&
				root.length > w.uri.fsPath.length &&
				(await fs.pathExists(w.uri.fsPath))
			) {
				root = w.uri.fsPath;
				rootWorkspace = w;
			}
		}

		return rootWorkspace;
	}
}

function getWorkspaceFolders(): readonly vscode.WorkspaceFolder[] {
	return vscode.workspace.workspaceFolders ?? [];
}

/*
 * Select a workspace folder to act on
 *
 * - If 0 workspaces are open, returns `undefined` after showing an error message.
 * - If 1 workspace is open, returns it.
 * - If >1 workspaces are open, shows the user a quick-pick menu to select their preference.
 */
export async function selectWorkspaceFolder(): Promise<
	vscode.WorkspaceFolder | undefined
> {
	const workspaceFolders = vscode.workspace.workspaceFolders;

	if (!workspaceFolders || workspaceFolders.length === 0) {
		vscode.window.showErrorMessage(
			"You must be inside a workspace to perform this action.",
		);
		return undefined;
	}

	if (workspaceFolders.length === 1) {
		return workspaceFolders[0];
	}

	// `undefined` if the user dismisses the quick pick
	const workspaceFolder =
		await selectWorkspaceFolderFromQuickPick(workspaceFolders);

	return workspaceFolder;
}

async function selectWorkspaceFolderFromQuickPick(
	workspaceFolders: readonly vscode.WorkspaceFolder[],
): Promise<vscode.WorkspaceFolder | undefined> {
	const workspaceFolderNames = workspaceFolders.map(
		(workspaceFolder) => workspaceFolder.name,
	);

	const workspaceFolderName = await vscode.window.showQuickPick(
		workspaceFolderNames,
		{
			canPickMany: false,
			title: "Select a workspace folder",
		},
	);

	if (!workspaceFolderName) {
		return undefined;
	}

	for (let workspaceFolder of workspaceFolders) {
		if (workspaceFolder.name === workspaceFolderName) {
			return workspaceFolder;
		}
	}

	// Should never get here
	output.log(
		`Matched a workspace folder name, but unexpectedly can't find corresponding workspace folder. Folder name: ${workspaceFolderName}.`,
	);
	return undefined;
}

/**
 * Save all open dirty editor tabs relevant to the workspace folder
 *
 * - Filters to only tabs living under the chosen workspace folder
 *   (note that this rules out untitled editors)
 * - Asks the user if they are okay with us saving the editor tabs
 */
export async function saveAllDirtyWorkspaceTextDocuments(
	workspaceFolder: vscode.WorkspaceFolder,
): Promise<boolean> {
	const textDocuments = dirtyWorkspaceTextDocuments(workspaceFolder);

	if (textDocuments.length === 0) {
		return true;
	}

	const answer = await vscode.window.showInformationMessage(
		`All editors within the ${workspaceFolder.name} workspace folder must be saved first. Proceed with saving these editors?`,
		{ modal: true },
		"Yes",
		"No",
	);

	if (answer !== "Yes") {
		return false;
	}

	const savedPromises = textDocuments.map((textDocument) =>
		textDocument.save(),
	);
	const saved = await Promise.all(savedPromises);
	return saved.every((save) => save);
}

function dirtyWorkspaceTextDocuments(
	workspaceFolder: vscode.WorkspaceFolder,
): vscode.TextDocument[] {
	return vscode.workspace.textDocuments.filter((document) => {
		if (document.isClosed) {
			// Not synchronized; refreshed from disk when reopened
			return false;
		}

		if (!document.isDirty) {
			return false;
		}

		if (document.isUntitled) {
			// Not part of any workspace folder
			return false;
		}

		if (!document.uri.fsPath.startsWith(workspaceFolder.uri.fsPath)) {
			return false;
		}

		return true;
	});
}
