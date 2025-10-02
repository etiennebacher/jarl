import * as vscode from "vscode";
import * as fs from "fs";
import which from "which";

import * as output from "./output";
import { FLIR_BINARY_NAME, BUNDLED_FLIR_EXECUTABLE } from "./constants";

export type ExecutableStrategy = "bundled" | "environment" | "path";

export async function resolveFlirBinaryPath(
	executableStrategy: ExecutableStrategy,
	executablePath?: string,
): Promise<string> {
	if (!vscode.workspace.isTrusted) {
		output.log(
			`Workspace is not trusted, using bundled executable: ${BUNDLED_FLIR_EXECUTABLE}`,
		);

		const bundledPath = flirBinaryFromBundled();

		if (bundledPath) {
			output.log(`Using bundled executable: ${bundledPath}`);
			return bundledPath;
		}

		throw new Error(
			"Workspace is not trusted and failed to find executable in bundled location",
		);
	} else if (executableStrategy === "bundled") {
		const bundledPath = flirBinaryFromBundled();

		if (bundledPath) {
			output.log(`Using bundled executable: ${bundledPath}`);
			return bundledPath;
		}

		output.log(
			"Bundled executable not found, falling back to environment executable",
		);
		const environmentPath = await flirBinaryFromEnvironment();

		if (environmentPath) {
			output.log(`Using environment executable: ${environmentPath}`);
			return environmentPath;
		}

		throw new Error(
			"Failed to find bundled executable and fallback environment executable",
		);
	} else if (executableStrategy === "environment") {
		const environmentPath = await flirBinaryFromEnvironment();

		if (environmentPath) {
			output.log(`Using environment executable: ${environmentPath}`);
			return environmentPath;
		}

		output.log(
			"Environment executable not found, falling back to bundled executable",
		);
		const bundledPath = flirBinaryFromBundled();

		if (bundledPath) {
			output.log(`Using bundled executable: ${bundledPath}`);
			return bundledPath;
		}

		throw new Error(
			"Failed to find environment executable and fallback bundled executable",
		);
	} else if (executableStrategy === "path") {
		const path = flirBinaryFromPath(executablePath);

		if (path) {
			output.log(`Using executable from \`flir.executablePath\`: ${path}`);
			return path;
		}

		throw new Error("Failed to find executable at `flir.executablePath`");
	} else {
		throw new Error("Unreachable");
	}
}

function flirBinaryFromBundled(): string | undefined {
	if (!fs.existsSync(BUNDLED_FLIR_EXECUTABLE)) {
		output.log(`Failed to find bundled executable: ${BUNDLED_FLIR_EXECUTABLE}`);
		return undefined;
	}

	output.log(`Found bundled executable: ${BUNDLED_FLIR_EXECUTABLE}`);
	return BUNDLED_FLIR_EXECUTABLE;
}

async function flirBinaryFromEnvironment(): Promise<string | undefined> {
	const environmentPath = await which(FLIR_BINARY_NAME, { nothrow: true });

	if (!environmentPath) {
		output.log("Failed to find environment executable");
		return undefined;
	}

	output.log(`Found environment executable: ${environmentPath}`);
	return environmentPath;
}

function flirBinaryFromPath(executablePath?: string): string | undefined {
	if (!executablePath) {
		output.log(
			"Failed to find executable from path, no `flir.executablePath` provided",
		);
		return undefined;
	}

	if (!fs.existsSync(executablePath)) {
		output.log(
			"Failed to find executable from path, provided `flir.executablePath` does not exist",
		);
		return undefined;
	}

	output.log(`Found executable from \`flir.executablePath\`: ${executablePath}`);
	return executablePath;
}
