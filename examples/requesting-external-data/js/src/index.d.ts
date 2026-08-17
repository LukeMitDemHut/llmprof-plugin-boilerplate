declare module "main" {
  export function ping(): I32;
  export function on_install(): I32;
  export function on_uninstall(): I32;
}

declare module "extism:host" {
  interface user {
    call_host_capability(ptr: I64): I64;
    read_file(ptr: I64): I64;
    write_file(ptr: I64): I64;
    list_files(ptr: I64): I64;
    delete_file(ptr: I64): I64;
  }
}

// Extism JS PDK built-in functions
declare function Input(): string | null;
declare function Output(data: string): void;
declare function fetch(url: string, options?: any): any;