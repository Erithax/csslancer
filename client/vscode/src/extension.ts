import {
    type ExtensionContext,
    workspace,
    window,
    commands,
    ViewColumn,
    Uri,
    WorkspaceConfiguration,
} from "vscode";
import * as path from "path";
import * as child_process from "child_process";

import {
    LanguageClient,
    type LanguageClientOptions,
    type ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined = undefined;

export function activate(context: ExtensionContext): Promise<void> {
    window.showInformationMessage(`Starting CSSLancer`);
    return startClient(context).catch((e) => {
        void window.showErrorMessage(`Failed to activate csslancer: ${e}`);
        throw e;
    });
}

async function startClient(context: ExtensionContext): Promise<void> {
    const config = workspace.getConfiguration("csslancer");
    const serverCommand = getServer(config);
    const run = {
        command: serverCommand,
        options: { env: Object.assign({}, process.env, { RUST_BACKTRACE: "1" }) },
    };
    const serverOptions: ServerOptions = {
        run,
        debug: run,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: "file", language: "csslancer" },
            { scheme: "file", language: "css"}
        ],
        initializationOptions: config,
    };

    client = new LanguageClient("csslancer", "CssLancer", serverOptions, clientOptions);

    client.start();
}

export function deactivate(): Promise<void> | undefined {
    return client?.stop();
}

function getServer(conf: WorkspaceConfiguration): string {
const pathInConfig = conf.get<string | null>("serverPath");
    if (pathInConfig !== undefined && pathInConfig !== null && pathInConfig !== "") {

        const validation = validateServer(pathInConfig);
        if (!validation.valid) {
            throw new Error(
                `\`csslancer.serverPath\` (${pathInConfig}) does not point to a valid csslancer binary:\n${(validation as {valid: boolean, message:string}).message}`
            );
        }
        return pathInConfig;
    }
    const windows = process.platform === "win32";
    const suffix = windows ? ".exe" : "";
    const binaryName = "csslancer" + suffix;

    const bundledPath = path.resolve(__dirname, binaryName);

    const bundledValidation = validateServer(bundledPath);
    if (bundledValidation.valid) {
        return bundledPath;
    }

    const binaryValidation = validateServer(binaryName);
    if (binaryValidation.valid) {
        return binaryName;
    }

    // TODO: validateServer fails at executing cmd `csslancer.exe` even when it's in PATH correctly
    return binaryName

    throw new Error(
        `Could not find a valid csslancer binary.\nBundled: ${(bundledValidation as {valid: boolean, message:string}).message}\nIn PATH: ${(binaryValidation as {valid: boolean, message:string}).message}`
    );
}

function validateServer(path: string): { valid: true } | { valid: false; message: string } {
    try {
        const result = child_process.spawnSync(path);
        // TODO: spawn returns 1 even on path="csslancer.exe" even though this command it correctly in PATH
        // tried suggestions https://stackoverflow.com/questions/27688804/how-do-i-debug-error-spawn-enoent-on-node-js
        if (result.status === 0) {
            return { valid: true };
        } else {
            const statusMessage = result.status !== null ? [`return status: ${result.status}`] : [];
            const errorMessage =
                result.error?.message !== undefined ? [`error: ${result.error.message}`] : [];
            const messages = [statusMessage, errorMessage];
            const messageSuffix =
                messages.length !== 0 ? `:\n\t${messages.flat().join("\n\t")}` : "";
            const message = `Failed to launch '${path}'${messageSuffix}`;
            return { valid: false, message };
        }
    } catch (e) {
        if (e instanceof Error) {
            return { valid: false, message: `Failed to launch '${path}': ${e.message}` };
        } else {
            return { valid: false, message: `Failed to launch '${path}': ${JSON.stringify(e)}` };
        }
    }
}
