declare module "main" {
  export function summarize(): I32;
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
