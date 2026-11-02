import * as vscode from "vscode";
import * as lc from "vscode-languageclient/node";
import { normalizePath } from "../utils";

export interface SyncFileSettingsParams {
	file_settings: FileSettings[];
}

export interface FileSettings {
	url: string;
	format: FileFormatSettings;
}

export interface FileFormatSettings {
	indent_style: "tab" | "space";
	indent_width: number;
	line_width: number;
}

export const SYNC_FILE_SETTINGS =
	new lc.NotificationType<SyncFileSettingsParams>("air/syncFileSettings");

export class FileSettingsState {
	// There is no way to propagate settings to all editors from the
	// notification handler as we can only change visible editors. To properly
	// handle background editors (e.g. editors in other groups), we need to
	// maintain a map of file settings and query it when the visibility state of
	// an editor changes.
	private settings = new Map<string, FileFormatSettings>();

	constructor(context: vscode.ExtensionContext) {
		context.subscriptions.push(
			vscode.window.onDidChangeActiveTextEditor(async (editor) => {
				if (editor) {
					this.apply(editor);
				}
			}),
		);
	}

	public handleSettingsNotification(params: SyncFileSettingsParams) {
		this.settings.clear();

		for (const fileSettings of params.file_settings) {
			const path = normalizePath(fileSettings.url);
			this.settings.set(path, fileSettings.format);
		}

		// Other editors pick up settings when they become active
		if (vscode.window.activeTextEditor) {
			this.apply(vscode.window.activeTextEditor);
		}
	}

	public apply(editor: vscode.TextEditor) {
		const uri = editor.document.uri;
		const settings = this.settings.get(uri.path);

		if (settings) {
			const insertSpaces = settings.indent_style === "space";
			const indentSize = settings.indent_width;

			let tabSize;

			if (insertSpaces) {
				tabSize = indentSize;
			} else {
				// With tabs, `tabSize` is purely visual and may differ from
				// `indentSize`, which drives formatting and line width. Prefer
				// the user's configured `tabSize`: the editor value may be one
				// we set earlier and is treated as ephemeral.
				const config = vscode.workspace.getConfiguration(undefined, {
					uri,
					languageId: "r",
				});
				tabSize =
					config.get<number>("editor.tabSize") ??
					editor.options.tabSize;
			}

			editor.options = {
				...editor.options,
				insertSpaces,
				indentSize,
				tabSize,
			};
		}
	}
}
