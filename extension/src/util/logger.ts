import * as vscode from "vscode";
import { redact } from "./redact";

export type Logger = Readonly<{
  info: (message: string) => void;
  warn: (message: string) => void;
  error: (message: string) => void;
  debug: (message: string) => void;
  dispose: () => void;
}>;

export function createLogger(name = "Lightning Git"): Logger {
  const channel = vscode.window.createOutputChannel(name);

  const write = (level: string, message: string) => {
    const line = `[${new Date().toISOString()}] ${level} ${redact(message)}`;
    channel.appendLine(line);
  };

  return {
    info: (message) => write("INFO", message),
    warn: (message) => write("WARN", message),
    error: (message) => write("ERROR", message),
    debug: (message) => write("DEBUG", message),
    dispose: () => channel.dispose()
  };
}