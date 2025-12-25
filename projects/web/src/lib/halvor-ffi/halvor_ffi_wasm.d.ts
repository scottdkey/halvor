// Type declarations for WASM module
// This file will be replaced by wasm-pack when the module is built
import type { HalvorWasmModule } from './generated-bindings';

declare module './halvor_ffi_wasm.js' {
  // wasm-pack generates a default export that initializes the module
  export default function init(): Promise<HalvorWasmModule>;

  // Named exports from the WASM module
  export const discoverAgents: () => Promise<any[]>;
  export const discoverViaTailscale: () => Promise<any[]>;
  export const discoverViaLocalNetwork: () => Promise<any[]>;
  export const pingAgent: (host: string, port: number) => Promise<boolean>;
  export const getHostInfo: (host: string, port: number) => Promise<any>;
  export const executeCommand: (
    host: string,
    port: number,
    command: string,
    args: string[]
  ) => Promise<string>;
}
