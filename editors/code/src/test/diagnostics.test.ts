import * as vscode from "vscode";
import * as assert from "assert";
import { suite, suiteSetup, test } from "mocha";

import { hasRule, waitForDiagnostics } from "./editor-utils";
import { waitLsp, withFile, withToml } from "./extension";

// Two violations of different rules, so that tests checking that a rule is
// not reported can first wait for the other one
const TWO_VIOLATIONS = "any(is.na(x))\nlength(x == 1)\n";

suite("Diagnostics Test Suite", () => {
	suiteSetup(waitLsp);

	test("Report violations with rule name, source and range", async () => {
		await withFile("x <- c(1, NA)\nany(is.na(x))\n", "R", async (_, doc) => {
			const diagnostics = await waitForDiagnostics(doc.uri, (d) =>
				hasRule(d, "any_is_na"),
			);

			assert.strictEqual(diagnostics.length, 1);
			const [diagnostic] = diagnostics;
			assert.strictEqual(diagnostic.source, "Jarl");
			assert.strictEqual(
				diagnostic.severity,
				vscode.DiagnosticSeverity.Warning,
			);
			assert.ok(diagnostic.range.isEqual(new vscode.Range(1, 0, 1, 13)));
			assert.match(diagnostic.message, /anyNA/);
		});
	});

	test("Update diagnostics when the document is saved", async () => {
		await withFile(TWO_VIOLATIONS, "R", async (editor, doc) => {
			await waitForDiagnostics(
				doc.uri,
				(d) => hasRule(d, "any_is_na") && hasRule(d, "length_test"),
			);

			await editor.edit((builder) =>
				builder.replace(doc.lineAt(0).range, "anyNA(x)"),
			);
			// The server only lints on open and save
			await doc.save();

			const diagnostics = await waitForDiagnostics(
				doc.uri,
				(d) => !hasRule(d, "any_is_na"),
			);
			assert.deepStrictEqual(
				diagnostics.map((d) => d.code),
				["length_test"],
			);
		});
	});

	test("Respect rules ignored in jarl.toml", async () => {
		const toml = `
[lint]
ignore = ["any_is_na"]
`;
		await withToml(toml, async () => {
			await withFile(TWO_VIOLATIONS, "R", async (_, doc) => {
				const diagnostics = await waitForDiagnostics(doc.uri, (d) =>
					hasRule(d, "length_test"),
				);
				assert.ok(!hasRule(diagnostics, "any_is_na"));
			});
		});
	});

	test("Respect rules selected in jarl.toml", async () => {
		const toml = `
[lint]
select = ["any_is_na"]
`;
		await withToml(toml, async () => {
			await withFile(TWO_VIOLATIONS, "R", async (_, doc) => {
				const diagnostics = await waitForDiagnostics(doc.uri, (d) =>
					hasRule(d, "any_is_na"),
				);
				assert.ok(!hasRule(diagnostics, "length_test"));
			});
		});
	});

	test("Respect jarl-ignore comments", async () => {
		const contents = "# jarl-ignore any_is_na: <reason>\n" + TWO_VIOLATIONS;
		await withFile(contents, "R", async (_, doc) => {
			const diagnostics = await waitForDiagnostics(doc.uri, (d) =>
				hasRule(d, "length_test"),
			);
			assert.ok(!hasRule(diagnostics, "any_is_na"));
		});
	});

	for (const ext of ["qmd", "Rmd"]) {
		test(`Lint R chunks in ${ext} documents`, async () => {
			const contents = `---
title: test
---

Some text with any(is.na(x)).

\`\`\`{r}
any(is.na(x))
\`\`\`
`;
			await withFile(contents, ext, async (_, doc) => {
				const diagnostics = await waitForDiagnostics(doc.uri, (d) =>
					hasRule(d, "any_is_na"),
				);

				// Only the code in the chunk is linted
				assert.strictEqual(diagnostics.length, 1);
				assert.strictEqual(diagnostics[0].range.start.line, 7);
			});
		});
	}
});
