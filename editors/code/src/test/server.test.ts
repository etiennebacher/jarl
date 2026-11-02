import * as vscode from "vscode";
import * as assert from "assert";
import { suite, suiteSetup, test } from "mocha";

import { hasRule, waitForDiagnostics } from "./editor-utils";
import { extension, internalApi, waitLsp, withFile } from "./extension";

suite("Server Test Suite", () => {
	suiteSetup(waitLsp);

	test("Activate extension and start server", async () => {
		assert.ok(extension().isActive);
		assert.ok(internalApi().ctx.lsp.getClient().isRunning());
	});

	test("Restart server with `jarl.restart`", async () => {
		await withFile("any(is.na(x))\n", "R", async (_, doc) => {
			await waitForDiagnostics(doc.uri, (d) => hasRule(d, "any_is_na"));

			const oldClient = internalApi().ctx.lsp.getClient();
			await vscode.commands.executeCommand("jarl.restart");
			const newClient = internalApi().ctx.lsp.getClient();

			assert.notStrictEqual(newClient, oldClient);
			assert.ok(!oldClient.isRunning());
			assert.ok(newClient.isRunning());

			// Open documents are linted again by the new server
			await waitForDiagnostics(doc.uri, (d) => hasRule(d, "any_is_na"));
		});
	});
});
