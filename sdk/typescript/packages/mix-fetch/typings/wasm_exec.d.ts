declare module '@nymproject/mix-fetch-wasm/wasm_exec' {
  export declare global {
    class Go {
      constructor();

      importObject: any;

      run(goWasm: any);
    }
  }
}
