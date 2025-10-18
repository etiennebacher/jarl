import * as vscode from "vscode";
import * as fs from "fs";
import which from "which";

import * as output from "./output";
import { JARL_BINARY_NAME, BUNDLED_JARL_EXECUTABLE } from "./constants";

export type ExecutableStrategy = "bundled" | "environment" | "path";

export async function resolveJarlBinaryPath(
	executableStrategy: ExecutableStrategy,
	executablePath?: string,
): Promise<string> {
	if (!vscode.workspace.isTrusted) {
		output.log(
			`Workspace is not trusted, using bundled executable: ${BUNDLED_JARL_EXECUTABLE}`,
		);

		const bundledPath = jarlBinaryFromBundled();

		if (bundledPath) {
			output.log(`Using bundled executable: ${bundledPath}`);
			return bundledPath;
		}

		throw new Error(
			"Workspace is not trusted and failed to find executable in bundled location",
		);
	} else if (executableStrategy === "bundled") {
		const bundledPath = jarlBinaryFromBundled();

		if (bundledPath) {
			output.log(`Using bundled executable: ${bundledPath}`);
			return bundledPath;
		}

		output.log(
			"Bundled executable not found, falling back to environment executable",
		);
		const environmentPath = await jarlBinaryFromEnvironment();

		if (environmentPath) {
			output.log(`Using environment executable: ${environmentPath}`);
			return environmentPath;
		}

		throw new Error(
			"Failed to find bundled executable and fallback environment executable",
		);
	} else if (executableStrategy === "environment") {
		const environmentPath = await jarlBinaryFromEnvironment();

		if (environmentPath) {
			output.log(`Using environment executable: ${environmentPath}`);
			return environmentPath;
		}

		output.log(
			"Environment executable not found, falling back to bundled executable",
		);
		const bundledPath = jarlBinaryFromBundled();

		if (bundledPath) {
			output.log(`Using bundled executable: ${bundledPath}`);
			return bundledPath;
		}

		throw new Error(
			"Failed to find environment executable and fallback bundled executable",
		);
	} else if (executableStrategy === "path") {
		const path = jarlBinaryFromPath(executablePath);

		if (path) {
			output.log(`Using executable from \`jarl.executablePath\`: ${path}`);
			return path;
		}

		throw new Error("Failed to find executable at `jarl.executablePath`");
	} else {
		throw new Error("Unreachable");
	}
}

function jarlBinaryFromBundled(): string | undefined {
	if (!fs.existsSync(BUNDLED_JARL_EXECUTABLE)) {
		output.log(`Failed to find bundled executable: ${BUNDLED_JARL_EXECUTABLE}`);
		return undefined;
	}

	output.log(`Found bundled executable: ${BUNDLED_JARL_EXECUTABLE}`);
	return BUNDLED_JARL_EXECUTABLE;
}

async function jarlBinaryFromEnvironment(): Promise<string | undefined> {
	const environmentPath = await which(JARL_BINARY_NAME, { nothrow: true });

	if (!environmentPath) {
		output.log("Failed to find environment executable");
		return undefined;
	}

	output.log(`Found environment executable: ${environmentPath}`);
	return environmentPath;
}

function jarlBinaryFromPath(executablePath?: string): string | undefined {
	if (!executablePath) {
		output.log(
			"Failed to find executable from path, no `jarl.executablePath` provided",
		);
		return undefined;
	}

	if (!fs.existsSync(executablePath)) {
		output.log(
			"Failed to find executable from path, provided `jarl.executablePath` does not exist",
		);
		return undefined;
	}

	output.log(`Found executable from \`jarl.executablePath\`: ${executablePath}`);
	return executablePath;
}
