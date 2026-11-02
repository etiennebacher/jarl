import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";

import { JARL_EXTENSION_ID, EXTENSION_ROOT_DIR } from "../constants";
import { createRandomFile, withFileEditor } from "./editor-utils";

// The test folder is the test workspace. Its `jarl.toml` overrides the one at
// the repo root, which excludes `**/test/**` from linting.
export const TEST_PATH = path.join(EXTENSION_ROOT_DIR, "src", "test");

export function testPath(file: string): string {
	return path.join(TEST_PATH, file);
}

export function extension() {
	const extension = vscode.extensions.getExtension(JARL_EXTENSION_ID);

	if (extension === undefined) {
		throw new Error(`Extension ${JARL_EXTENSION_ID} not found`);
	}

	return extension;
}

export function api() {
	return extension().exports;
}

export function internalApi() {
	return extension().exports.__private;
}

export async function waitLsp() {
	// Make sure jarl-vscode is activated as activation events might not have
	// been triggered
	await extension().activate();

	// No-op if LSP is already started. Doesn't run concurrently with the `start()`
	// task started by `activate()` so is safe to use to wait until the LSP is fully
	// started up.
	await internalApi().ctx.lsp.start();
}

// The server reads `jarl.toml` on every lint, so documents opened in `run`
// pick up `content`
export async function withToml(
	content: string,
	run: () => Promise<void>,
): Promise<void> {
	const tomlPath = testPath("jarl.toml");
	const original = fs.readFileSync(tomlPath, "utf8");
	fs.writeFileSync(tomlPath, content);

	try {
		await run();
	} finally {
		fs.writeFileSync(tomlPath, original);
	}
}

// Files must live in the test workspace for the server to find `jarl.toml`
export async function withFile(
	contents: string,
	fileExtension: string,
	run: (editor: vscode.TextEditor, doc: vscode.TextDocument) => Promise<void>,
): Promise<void> {
	const uri = await createRandomFile(contents, fileExtension, TEST_PATH);

	try {
		await withFileEditor(uri.fsPath, run);
	} finally {
		fs.unlinkSync(uri.fsPath);
	}
}
