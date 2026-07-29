/* tslint:disable */
/* eslint-disable */

export function atbash(input: string): string;

export function base32_decode(input: string): string;

export function base32_encode(input: string): string;

export function base58_decode(input: string): string;

export function base58_encode(input: string): string;

export function base64url_decode(input: string): string;

export function base64url_encode(input: string): string;

export function br_document(kind: string, action: string, input: string, uf: string, random: Uint8Array): string;

export function caesar(input: string, shift: number): string;

export function crc32(input: string): string;

export function fnv1a32(input: string): string;

export function fnv1a64(input: string): string;

export function hex_decode(input: string): string;

export function hex_encode(input: string): string;

export function inspect_ksuid(input: string): string;

export function inspect_ulid(input: string): string;

export function inspect_uuid_v7(input: string): string;

export function ksuid(timestamp_secs: number, payload: Uint8Array): string;

export function leet_decode(input: string): string;

export function leet_encode(input: string): string;

export function morse_decode(input: string): string;

export function morse_encode(input: string): string;

export function password_generate(length: number, count: number, lowercase: boolean, uppercase: boolean, numbers: boolean, symbols: boolean, exclude_similar: boolean, exclude_ambiguous: boolean, require_each: boolean, random: Uint8Array): string;

export function phone_format(input: string): string;

export function phone_lookup(input: string): string;

export function polybius_decode(input: string): string;

export function polybius_encode(input: string): string;

export function qr_matrix(text: string, ecc: string): string;

export function rot13(input: string): string;

export function ulid(timestamp_ms: number, random: Uint8Array): string;

export function uuid_v7(timestamp_ms: number, random: Uint8Array): string;

export function vigenere(input: string, key: string, decode: boolean): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly atbash: (a: number, b: number, c: number) => void;
    readonly base32_decode: (a: number, b: number, c: number) => void;
    readonly base32_encode: (a: number, b: number, c: number) => void;
    readonly base58_decode: (a: number, b: number, c: number) => void;
    readonly base58_encode: (a: number, b: number, c: number) => void;
    readonly base64url_decode: (a: number, b: number, c: number) => void;
    readonly base64url_encode: (a: number, b: number, c: number) => void;
    readonly br_document: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => void;
    readonly caesar: (a: number, b: number, c: number, d: number) => void;
    readonly crc32: (a: number, b: number, c: number) => void;
    readonly fnv1a32: (a: number, b: number, c: number) => void;
    readonly fnv1a64: (a: number, b: number, c: number) => void;
    readonly hex_decode: (a: number, b: number, c: number) => void;
    readonly hex_encode: (a: number, b: number, c: number) => void;
    readonly inspect_ksuid: (a: number, b: number, c: number) => void;
    readonly inspect_ulid: (a: number, b: number, c: number) => void;
    readonly inspect_uuid_v7: (a: number, b: number, c: number) => void;
    readonly ksuid: (a: number, b: number, c: number, d: number) => void;
    readonly leet_decode: (a: number, b: number, c: number) => void;
    readonly leet_encode: (a: number, b: number, c: number) => void;
    readonly morse_decode: (a: number, b: number, c: number) => void;
    readonly morse_encode: (a: number, b: number, c: number) => void;
    readonly password_generate: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => void;
    readonly phone_format: (a: number, b: number, c: number) => void;
    readonly phone_lookup: (a: number, b: number, c: number) => void;
    readonly polybius_decode: (a: number, b: number, c: number) => void;
    readonly polybius_encode: (a: number, b: number, c: number) => void;
    readonly qr_matrix: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly rot13: (a: number, b: number, c: number) => void;
    readonly ulid: (a: number, b: number, c: number, d: number) => void;
    readonly uuid_v7: (a: number, b: number, c: number, d: number) => void;
    readonly vigenere: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
