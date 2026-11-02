import * as vscode from "vscode";
import * as lc from "vscode-languageclient/node";
import { default as PQueue } from "p-queue";
import { getInitializationOptions, getWorkspaceSettings } from "./settings";
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

	// We use the same output channel for all LSP instances (e.g. a new instance
	// after a restart) to avoid having multiple channels in the Output viewpane.
	private channel: vscode.OutputChannel;

	private state = State.Stopped;
	private stateQueue: PQueue;

	constructor(context: vscode.ExtensionContext) {
		this.channel = vscode.window.createOutputChannel(
			"Jarl Language Server",
		);
		context.subscriptions.push(this.channel, registerLogger(this.channel));

		this.stateQueue = new PQueue({ concurrency: 1 });
	}

	public getClient(): lc.LanguageClient {
		if (!this.client) {
			throw new Error("LSP must be started");
		}
		return this.client;
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

		let clientOptions: lc.LanguageClientOptions = {
			documentSelector: [
				// R files
				{ language: "r", scheme: "file" },
				{ language: "r", pattern: "**/*.{r,R}" },
				// R Markdown files (language ID registered by REditorSupport.r or Positron)
				{ language: "rmd", scheme: "file" },
				{ pattern: "**/*.{Rmd,rmd}", scheme: "file" },
				// Quarto files (language ID registered by the Quarto extension or Positron)
				{ language: "quarto", scheme: "file" },
				{ pattern: "**/*.{qmd,Qmd}", scheme: "file" },
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

		await client.start();

		// Only update state if no error occurred
		this.client = client;
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
		}
	}

	private async restartImpl() {
		if (this.state === State.Started) {
			await this.stopImpl();
		}
		await this.startImpl();
	}
}
