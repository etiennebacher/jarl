import * as path from "path";

const folderName = path.basename(__dirname);

/**
 * ID of the extension on the marketplaces.
 * Needed to access the extension from the `vscode` API.
 */
export const FLIR_EXTENSION_ID = "etiennebacher.flir-vscode";

/**
 * Path to the root directory of this extension.
 * https://github.com/microsoft/vscode-python-tools-extension-template/blob/main/src/common/constants.ts
 */
export const EXTENSION_ROOT_DIR =
	folderName === "common"
		? path.dirname(path.dirname(__dirname))
		: path.dirname(__dirname);

/**
 * Name of the `flir` binary based on the current platform.
 */
export const FLIR_BINARY_NAME = process.platform === "win32" ? "flir.exe" : "flir";

/**
 * Path to the `flir` executable that is bundled with the extension.
 * The GitHub Action is in charge of placing the executable here.
 */
export const BUNDLED_FLIR_EXECUTABLE = path.join(
	EXTENSION_ROOT_DIR,
	"bundled",
	"bin",
	FLIR_BINARY_NAME,
);
