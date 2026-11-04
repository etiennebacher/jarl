import * as vscode from "vscode";
import * as assert from "assert";
import { suite, suiteSetup, test } from "mocha";

import { hasRule, waitForDiagnostics } from "./editor-utils";
import { waitLsp, withFile } from "./extension";

const VIOLATION = "x <- c(1, NA)\nany(is.na(x))\n";

async function codeActions(
	doc: vscode.TextDocument,
	range: vscode.Range,
): Promise<vscode.CodeAction[]> {
	return await vscode.commands.executeCommand<vscode.CodeAction[]>(
		"vscode.executeCodeActionProvider",
		doc.uri,
		range,
	);
}

async function waitForViolation(
	doc: vscode.TextDocument,
): Promise<vscode.Diagnostic> {
	const diagnostics = await waitForDiagnostics(doc.uri, (d) =>
		hasRule(d, "any_is_na"),
	);
	return diagnostics[0];
}

async function applyAction(
	doc: vscode.TextDocument,
	range: vscode.Range,
	title: string,
) {
	const actions = await codeActions(doc, range);
	const action = actions.find((a) => a.title === title);
	assert.ok(
		action?.edit,
		`No action "${title}" in ${JSON.stringify(actions.map((a) => a.title))}`,
	);
	assert.ok(await vscode.workspace.applyEdit(action.edit));
	// The server only lints on open and save
	await doc.save();
	return action;
}

suite("Code Actions Test Suite", () => {
	suiteSetup(waitLsp);

	test("Apply quick fix", async () => {
		await withFile(VIOLATION, "R", async (_, doc) => {
			const diagnostic = await waitForViolation(doc);

			const action = await applyAction(
				doc,
				diagnostic.range,
				"Fix: `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.",
			);
			assert.strictEqual(
				action.kind?.value,
				vscode.CodeActionKind.QuickFix.value,
			);
			assert.strictEqual(action.isPreferred, true);

			assert.strictEqual(doc.getText(), "x <- c(1, NA)\nanyNA(x)\n");
			await waitForDiagnostics(doc.uri, (d) => d.length === 0);
		});
	});

	test("Suppress violation with jarl-ignore comment", async () => {
		await withFile(VIOLATION, "R", async (_, doc) => {
			const diagnostic = await waitForViolation(doc);

			await applyAction(
				doc,
				diagnostic.range,
				"Suppress `any_is_na` violation with jarl-ignore comment.",
			);

			assert.strictEqual(
				doc.getText(),
				"x <- c(1, NA)\n# jarl-ignore any_is_na: <reason>\nany(is.na(x))\n",
			);
			await waitForDiagnostics(doc.uri, (d) => d.length === 0);
		});
	});

	test("Suppress violations in a Quarto chunk", async () => {
		const contents = "```{r}\nx <- c(1, NA)\nany(is.na(x))\n```\n";
		await withFile(contents, "qmd", async (_, doc) => {
			const diagnostic = await waitForViolation(doc);

			await applyAction(
				doc,
				diagnostic.range,
				"Ignore all violations of `any_is_na` in this chunk.",
			);

			assert.strictEqual(
				doc.getText(),
				"```{r}\n#| jarl-ignore-chunk:\n#|   - any_is_na: <reason>\nx <- c(1, NA)\nany(is.na(x))\n```\n",
			);
			await waitForDiagnostics(doc.uri, (d) => d.length === 0);
		});
	});

	test("Only offer chunk suppression in R Markdown and Quarto", async () => {
		await withFile(VIOLATION, "R", async (_, doc) => {
			const diagnostic = await waitForViolation(doc);
			const titles = (await codeActions(doc, diagnostic.range)).map(
				(a) => a.title,
			);
			assert.ok(!titles.some((t) => t.includes("in this chunk")));
		});
	});

	test("No code actions outside of violations", async () => {
		await withFile(VIOLATION, "R", async (_, doc) => {
			await waitForViolation(doc);
			const actions = await codeActions(doc, new vscode.Range(0, 0, 0, 0));
			assert.deepStrictEqual(
				actions.map((a) => a.title),
				[],
			);
		});
	});
});
