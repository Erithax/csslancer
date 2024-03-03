"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.deactivate = exports.activate = void 0;
const vscode_1 = require("vscode");
const path = require("path");
const child_process = require("child_process");
const node_1 = require("vscode-languageclient/node");
let client = undefined;
function activate(context) {
    vscode_1.window.showInformationMessage(`Starting CSSLancer`);
    return startClient(context).catch((e) => {
        void vscode_1.window.showErrorMessage(`Failed to activate csslancer: ${e}`);
        throw e;
    });
}
exports.activate = activate;
async function startClient(context) {
    const config = vscode_1.workspace.getConfiguration("csslancer");
    const serverCommand = getServer(config);
    const run = {
        command: serverCommand,
        options: { env: Object.assign({}, process.env, { RUST_BACKTRACE: "1" }) },
    };
    const serverOptions = {
        run,
        debug: run,
    };
    const clientOptions = {
        documentSelector: [
            { scheme: "file", language: "csslancer" },
            { scheme: "file", language: "css" }
        ],
        initializationOptions: config,
    };
    client = new node_1.LanguageClient("csslancer", "CssLancer", serverOptions, clientOptions);
    client.start();
}
function deactivate() {
    return client?.stop();
}
exports.deactivate = deactivate;
function getServer(conf) {
    const pathInConfig = conf.get("serverPath");
    if (pathInConfig !== undefined && pathInConfig !== null && pathInConfig !== "") {
        const validation = validateServer(pathInConfig);
        if (!validation.valid) {
            throw new Error(`\`csslancer.serverPath\` (${pathInConfig}) does not point to a valid csslancer binary:\n${validation.message}`);
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
    return binaryName;
    throw new Error(`Could not find a valid csslancer binary.\nBundled: ${bundledValidation.message}\nIn PATH: ${binaryValidation.message}`);
}
function validateServer(path) {
    try {
        const result = child_process.spawnSync(path);
        // TODO: spawn returns 1 even on path="csslancer.exe" even though this command it correctly in PATH
        // tried suggestions https://stackoverflow.com/questions/27688804/how-do-i-debug-error-spawn-enoent-on-node-js
        if (result.status === 0) {
            return { valid: true };
        }
        else {
            const statusMessage = result.status !== null ? [`return status: ${result.status}`] : [];
            const errorMessage = result.error?.message !== undefined ? [`error: ${result.error.message}`] : [];
            const messages = [statusMessage, errorMessage];
            const messageSuffix = messages.length !== 0 ? `:\n\t${messages.flat().join("\n\t")}` : "";
            const message = `Failed to launch '${path}'${messageSuffix}`;
            return { valid: false, message };
        }
    }
    catch (e) {
        if (e instanceof Error) {
            return { valid: false, message: `Failed to launch '${path}': ${e.message}` };
        }
        else {
            return { valid: false, message: `Failed to launch '${path}': ${JSON.stringify(e)}` };
        }
    }
}
//# sourceMappingURL=extension.js.map