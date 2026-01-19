import * as vscode from "vscode";
import * as lc from "vscode-languageclient/node";
import { default as PQueue } from "p-queue";
import { getInitializationOptions, getWorkspaceSettings } from "./settings";
import {
	FileSettingsState,
	SyncFileSettingsParams,
} from "./notification/sync-file-settings";
import { Middleware, ResponseError } from "vscode-languageclient/node";
import { SYNC_FILE_SETTINGS } from "./notification/sync-file-settings";
import { registerLogger } from "./output";
import { resolveJarlBinaryPath } from "./binary";
import { getRootWorkspaceFolder } from "./workspace";

// All session management operations are put on a queue. They can't run
// concurrently and either result in a started or stopped state. Starting when
// started is a noop, same for stopping when stopped. On the other hand
// restarting is always scheduled.
enum State {
	Started = "started",
	Stopped = "stopped",
}

export class Lsp {
	private client: lc.LanguageClient | null = null;

	private binaryPath: string | null = null;

	// We've received and processed an `jarl.toml` settings synchronization
	// notification. Used to synchronize unit tests with the LSP.
	private onSettingsNotification: vscode.Event<SyncFileSettingsParams>;

	// We use the same output channel for all LSP instances (e.g. a new instance
	// after a restart) to avoid having multiple channels in the Output viewpane.
	private channel: vscode.OutputChannel;

	private state = State.Stopped;
	private stateQueue: PQueue;

	private fileSettings: FileSettingsState;

	private onSettingsNotificationEmitter: vscode.EventEmitter<SyncFileSettingsParams>;

	constructor(context: vscode.ExtensionContext) {
		this.channel = vscode.window.createOutputChannel(
			"Jarl Language Server",
		);
		context.subscriptions.push(this.channel, registerLogger(this.channel));

		this.stateQueue = new PQueue({ concurrency: 1 });
		this.fileSettings = new FileSettingsState(context);

		this.onSettingsNotificationEmitter =
			new vscode.EventEmitter<SyncFileSettingsParams>();
		context.subscriptions.push(this.onSettingsNotificationEmitter);

		this.onSettingsNotification = this.onSettingsNotificationEmitter.event;

		this.onSettingsNotification((settings) =>
			this.fileSettings.handleSettingsNotification(settings),
		);
	}

	public getClient(): lc.LanguageClient {
		if (!this.client) {
			throw new Error("LSP must be started");
		}
		return this.client;
	}

	public getBinaryPath(): string {
		if (!this.binaryPath) {
			throw new Error("LSP must be started");
		}
		return this.binaryPath;
	}

	public waitForSettingsNotification(): Promise<void> {
		return new Promise((resolve, _) => {
			const disposable = this.onSettingsNotification(() => {
				disposable.dispose();
				resolve();
			});
		});
	}

	public async start() {
		await this.stateQueue.add(async () => await this.startImpl());
	}

	public async restart() {
		await this.stateQueue.add(async () => await this.restartImpl());
	}

	public async stop() {
		await this.stateQueue.add(async () => await this.stopImpl());
	}

	private async startImpl() {
		// Noop if already started
		if (this.state === State.Started) {
			return;
		}

		const workspaceFolder = await getRootWorkspaceFolder();

		const workspaceSettings = getWorkspaceSettings("jarl", workspaceFolder);
		const initializationOptions = getInitializationOptions("jarl");

		const binaryPath = await resolveJarlBinaryPath(
			workspaceSettings.executableStrategy,
			workspaceSettings.executablePath,
		);

		let serverOptions: lc.ServerOptions = {
			command: binaryPath,
			args: ["server"],
		};

		// Simplified middleware - remove complex configuration handling for now
		let middleware: Middleware = {};

		let clientOptions: lc.LanguageClientOptions = {
			// Look for R files only
			documentSelector: [
				{ language: "r", scheme: "file" },
				{ language: "r", pattern: "**/*.{r,R}" },
			],
			outputChannel: this.channel,
			initializationOptions: initializationOptions,
		};

		const client = new lc.LanguageClient(
			"jarlLanguageServer",
			"Jarl Language Server",
			serverOptions,
			clientOptions,
		);

		// Commenting out notifications for now to fix initialization issues
		// client.onNotification(SYNC_FILE_SETTINGS, (settings) => {
		// 	this.onSettingsNotificationEmitter.fire(settings);
		// });

		await client.start();

		// Only update state if no error occurred
		this.client = client;
		this.binaryPath = binaryPath;
		this.state = State.Started;
	}

	private async stopImpl() {
		// Noop if already stopped
		if (this.state === State.Stopped) {
			return;
		}

		try {
			await this.client?.stop();
		} finally {
			// We're always stopped even if an error happens. Hard to do better
			// in that case, we just drop the client and hope an eventual restart
			// will put us back in a good place.
			this.state = State.Stopped;
			this.client = null;
			this.binaryPath = null;
		}
	}

	private async restartImpl() {
		if (this.state === State.Started) {
			await this.stopImpl();
		}
		await this.startImpl();
	}
}
