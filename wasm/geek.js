const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

let corePromise;
let imageBitmap;

const toolExplanations = Object.freeze({
  "uuid-v4": "Creates 122 random identifier bits with the browser CSPRNG, sets the UUID version field to 4 and the RFC variant field, then formats the 128 bits as five hexadecimal groups.",
  "uuid-v7": "Combines the current Unix millisecond timestamp with 74 browser-random bits. Rust sets the version 7 and variant bit fields so the result sorts roughly by creation time.",
  "ulid": "Combines a 48-bit Unix millisecond timestamp with 80 random bits, then encodes the 128-bit value using Crockford Base32 into 26 sortable characters.",
  "ksuid": "Combines a 32-bit timestamp offset from the KSUID epoch with a 128-bit random payload, then Base62 encodes the 160-bit value into 27 sortable characters.",
  "time-convert": "Parses a Unix timestamp or date string in the browser. Numeric input is detected as seconds, milliseconds, microseconds or nanoseconds by magnitude, then converted through Date and Intl.",
  "random-bytes": "Uses browser crypto.getRandomValues() to fill a byte array with cryptographically strong random bytes, then formats those bytes as hex, Base64, Base64url or decimal values.",
  "random-integer": "Uses rejection sampling over browser-random 32-bit values so every integer in the requested inclusive range has the same probability.",
  "random-decimal": "Combines browser-random bytes into a 53-bit integer, divides by 2^53 to get a uniform fraction, then scales it into the requested numeric range.",
  "random-choice": "Splits the input into lines or comma-separated values, draws an unbiased random index with browser crypto and returns the selected item.",
  "image-editor": "Uses browser image decoding and Canvas drawing. Resize changes the target canvas, rotation and flips alter the draw transform, CSS canvas filters adjust color, and export serializes PNG, JPEG or WebP.",
  "password-generator": "The browser supplies cryptographic random bytes. Rust builds the selected character pool, removes excluded characters, guarantees required sets when enabled, picks characters with unbiased random indexes and shuffles the result.",
  "qr-generator": "Rust builds the QR module matrix with the selected error correction level. Canvas then paints dots, finder corners, background, optional logo and exports the chosen image format.",
  "phone-code-identifier": "Rust normalizes digits and country names, matches Brazilian DDD area codes and international calling prefixes, then returns compact records. JavaScript only attaches local flag assets.",
  "br-document-toolkit": "Rust ports the compact selo-style document logic for CPF, CNPJ, CNH, PIS/PASEP/NIS, RENAVAM, Titulo Eleitoral, CEP, Brazilian phone, license plate, CNS, RG-SP, IE-SP/MG/RS/PR, PIX keys and synthetic test people. JavaScript only renders controls and readable cards.",
  "inspect-uuid-v7": "Parses UUID text, validates the version and variant fields, then decodes the leading timestamp bits into Unix milliseconds and an ISO date.",
  "inspect-ulid": "Decodes Crockford Base32 into bytes, reads the first 48 bits as Unix milliseconds and reports the remaining random payload.",
  "inspect-ksuid": "Base62 decodes the 27 characters into 20 bytes, reads the first 4 bytes as seconds since the KSUID epoch and reports the payload.",
  "sha-256": "Uses Web Crypto to hash UTF-8 bytes. SHA-256 pads the message, processes 512-bit blocks through 64 compression rounds and returns a 256-bit one-way digest.",
  "sha-512": "Uses Web Crypto to hash UTF-8 bytes. SHA-512 pads the message, processes 1024-bit blocks through 80 compression rounds and returns a 512-bit one-way digest.",
  "base64-encode": "Converts UTF-8 bytes into 6-bit groups and maps each group to the Base64 alphabet. Padding marks incomplete final groups.",
  "base64-decode": "Maps Base64 characters back to 6-bit values, joins them into bytes, removes padding and decodes the bytes as UTF-8 text.",
  "url-encode": "Uses percent encoding for text that is unsafe in a URL component. UTF-8 bytes outside the safe set become %HH hexadecimal escapes.",
  "url-decode": "Reads %HH hexadecimal escapes from a URL component, rebuilds the original UTF-8 bytes and returns text.",
  "hex-encode": "Converts each UTF-8 byte to two hexadecimal digits, one digit for the high nibble and one for the low nibble.",
  "hex-decode": "Reads pairs of hexadecimal digits, converts each pair back into one byte and decodes the byte stream as UTF-8.",
  "base32-encode": "Groups input bytes into 5-bit chunks and maps those chunks to the RFC 4648 A-Z2-7 alphabet, adding padding when needed.",
  "base32-decode": "Maps RFC 4648 Base32 characters back to 5-bit chunks, joins them into bytes and decodes the result as UTF-8.",
  "base58-encode": "Treats the input bytes as one large integer, repeatedly divides by 58 and maps remainders to the Bitcoin Base58 alphabet while preserving leading zero bytes.",
  "base58-decode": "Treats Base58 text as a base-58 integer, multiplies and adds character indexes to rebuild bytes, then restores leading zero bytes.",
  "base64url-encode": "Encodes bytes like Base64, then uses - and _ instead of + and / and omits padding so the result is safe in URLs and tokens.",
  "base64url-decode": "Restores missing padding, maps - and _ back to the Base64 value set, decodes bytes and returns UTF-8 text.",
  "crc32": "Computes the IEEE CRC-32 checksum by folding each byte through a polynomial remainder table. It detects accidental corruption, not malicious changes.",
  "fnv1a-32": "Starts from the 32-bit FNV offset basis, XORs each byte into the hash and multiplies by the FNV prime with wrapping arithmetic.",
  "fnv1a-64": "Starts from the 64-bit FNV offset basis, XORs each byte into the hash and multiplies by the 64-bit FNV prime with wrapping arithmetic.",
  "rot13": "Rotates ASCII letters by 13 positions inside A-Z or a-z. Because the alphabet has 26 letters, applying ROT13 twice returns the original text.",
  "caesar": "Shifts each ASCII letter by the selected amount inside A-Z or a-z and leaves other characters unchanged. Negative or opposite shifts decode it.",
  "vigenere-encode": "Repeats the key over the message and applies a Caesar shift based on each key letter. Nonletters pass through without consuming key position.",
  "vigenere-decode": "Repeats the key over the ciphertext and subtracts each key-letter shift to reverse the Vigenere substitution.",
  "morse-encode": "Maps letters, digits and common punctuation to dot-dash tokens. Spaces become slash separators and characters are separated by spaces.",
  "morse-decode": "Splits Morse tokens by spaces, maps dot-dash patterns back to characters and treats slash tokens as word spaces.",
  "leet-encode": "Applies a simple character substitution table such as a to 4, e to 3, i to 1, o to 0 and s to 5.",
  "leet-decode": "Reverses the simple leet table by mapping common digit and symbol substitutions back to letters.",
  "atbash": "Mirrors each ASCII letter across the alphabet, so A becomes Z, B becomes Y and so on. The same operation encodes and decodes.",
  "polybius-encode": "Places letters in a 5 by 5 square with I and J sharing a cell, then replaces each letter with its row and column number.",
  "polybius-decode": "Reads row and column number pairs from the Polybius square and maps each pair back to a letter, using I for the shared I/J cell."
});

const toolSamples = Object.freeze({
  "time-convert": "1800000000",
  "random-bytes": "16",
  "random-integer": "1, 100",
  "random-decimal": "0, 1",
  "random-choice": "alpha\nbeta\ngamma",
  "inspect-uuid-v7": "015d3ef7-9800-7123-9678-90abcdef1011",
  "inspect-ulid": "01BMZFF60028T5CY4GNF6YY40H",
  "inspect-ksuid": "0ujtsYcgvSTl8PAuAdqWYSMnLOv",
  "sha-256": "hello",
  "sha-512": "hello",
  "base64-encode": "hello",
  "base64-decode": "aGVsbG8=",
  "url-encode": "hello world?x=1&y=2",
  "url-decode": "hello%20world%3Fx%3D1%26y%3D2",
  "hex-encode": "hello",
  "hex-decode": "68656c6c6f",
  "base32-encode": "hello",
  "base32-decode": "NBSWY3DP",
  "base58-encode": "hello",
  "base58-decode": "Cn8eVZg",
  "base64url-encode": "hello",
  "base64url-decode": "aGVsbG8",
  "crc32": "123456789",
  "fnv1a-32": "hello",
  "fnv1a-64": "hello",
  "rot13": "Hello, world!",
  "caesar": "Attack at dawn",
  "vigenere-encode": "ATTACKATDAWN",
  "vigenere-decode": "LXFOPVEFRNHR",
  "morse-encode": "SOS",
  "morse-decode": "... --- ...",
  "leet-encode": "hello",
  "leet-decode": "h3ll0",
  "atbash": "Hello, world!",
  "polybius-encode": "ABC",
  "polybius-decode": "11 12 13"
});

const seloSamples = Object.freeze({
  auto: "529.982.247-25",
  cpf: "529.982.247-25",
  cnpj: "12.ABC.345/01DE-35",
  cnh: "12345678900",
  pis: "120.01234.56-4",
  renavam: "12345678900",
  "voter-id": "000000000116",
  cep: "01310-100",
  "phone-br": "+55 (11) 93039-0628",
  plate: "ABC-1234",
  cns: "898001160000001",
  rg: "12.345.678-2",
  ie: "110.042.490.114",
  pix: "529.982.247-25",
  person: ""
});

const toolOptions = Object.freeze({
  "random-bytes": {
    option: "format",
    optionLabel: "Format",
    optionDefault: "hex",
    optionPlaceholder: "hex | base64 | base64url | decimal",
    optionHelp: "Output format for the generated bytes."
  },
  "random-decimal": {
    option: "digits",
    optionLabel: "Digits",
    optionDefault: "12",
    optionPlaceholder: "12",
    optionHelp: "Maximum fractional digits to show."
  },
  caesar: {
    option: "shift",
    optionLabel: "Shift",
    optionDefault: "13",
    optionPlaceholder: "13",
    optionHelp: "Number of alphabet positions to rotate each letter."
  },
  "vigenere-encode": {
    option: "key",
    optionLabel: "Key",
    optionDefault: "LEMON",
    optionPlaceholder: "LEMON",
    optionHelp: "Letters that repeat across the text and define each Caesar shift."
  },
  "vigenere-decode": {
    option: "key",
    optionLabel: "Key",
    optionDefault: "LEMON",
    optionPlaceholder: "LEMON",
    optionHelp: "Same key used for encoding; Rust subtracts those shifts to decode."
  }
});

export const seloKinds = Object.freeze([
  { id: "cpf", label: "CPF", sample: seloSamples.cpf },
  { id: "cnpj", label: "CNPJ", sample: seloSamples.cnpj },
  { id: "cnh", label: "CNH", sample: seloSamples.cnh },
  { id: "pis", label: "PIS/PASEP/NIS", sample: seloSamples.pis },
  { id: "renavam", label: "RENAVAM", sample: seloSamples.renavam },
  { id: "voter-id", label: "Titulo Eleitoral", sample: seloSamples["voter-id"] },
  { id: "cep", label: "CEP", sample: seloSamples.cep },
  { id: "phone-br", label: "Brazil phone", sample: seloSamples["phone-br"] },
  { id: "plate", label: "License plate", sample: seloSamples.plate },
  { id: "cns", label: "CNS", sample: seloSamples.cns },
  { id: "rg", label: "RG-SP", sample: seloSamples.rg },
  { id: "ie", label: "Inscricao Estadual", sample: seloSamples.ie },
  { id: "pix", label: "PIX key", sample: seloSamples.pix },
  { id: "auto", label: "Auto detect", sample: seloSamples.auto },
  { id: "person", label: "Synthetic person", sample: seloSamples.person }
]);

export const seloActions = Object.freeze([
  { id: "validate", label: "Validate" },
  { id: "format", label: "Format" },
  { id: "generate", label: "Generate" },
  { id: "detect", label: "Detect" }
]);

export const brazilUfs = Object.freeze([
  { code: "SP", name: "Sao Paulo" },
  { code: "AC", name: "Acre" },
  { code: "AL", name: "Alagoas" },
  { code: "AP", name: "Amapa" },
  { code: "AM", name: "Amazonas" },
  { code: "BA", name: "Bahia" },
  { code: "CE", name: "Ceara" },
  { code: "DF", name: "Distrito Federal" },
  { code: "ES", name: "Espirito Santo" },
  { code: "GO", name: "Goias" },
  { code: "MA", name: "Maranhao" },
  { code: "MT", name: "Mato Grosso" },
  { code: "MS", name: "Mato Grosso do Sul" },
  { code: "MG", name: "Minas Gerais" },
  { code: "PA", name: "Para" },
  { code: "PB", name: "Paraiba" },
  { code: "PR", name: "Parana" },
  { code: "PE", name: "Pernambuco" },
  { code: "PI", name: "Piaui" },
  { code: "RJ", name: "Rio de Janeiro" },
  { code: "RN", name: "Rio Grande do Norte" },
  { code: "RS", name: "Rio Grande do Sul" },
  { code: "RO", name: "Rondonia" },
  { code: "RR", name: "Roraima" },
  { code: "SC", name: "Santa Catarina" },
  { code: "SE", name: "Sergipe" },
  { code: "TO", name: "Tocantins" }
]);

export const tools = Object.freeze([
  { id: "uuid-v4", label: "UUID v4", origin: "browser", mode: "generate", hint: "Uses crypto.randomUUID()." },
  { id: "uuid-v7", label: "UUID v7", origin: "rust", mode: "generate", hint: "Browser random bytes + Rust RFC 9562 layout." },
  { id: "ulid", label: "ULID", origin: "rust", mode: "generate", hint: "Browser random bytes + Rust Crockford Base32 layout." },
  { id: "ksuid", label: "KSUID", origin: "rust", mode: "generate", hint: "Browser random bytes + Rust Base62 layout." },
  { id: "time-convert", label: "Time converter", origin: "browser", mode: "input", hint: "Unix seconds/ms/us/ns or date text via Date and Intl." },
  { id: "random-bytes", label: "Random bytes", origin: "browser", mode: "input", hint: "CSPRNG bytes formatted as hex, Base64, Base64url or decimal." },
  { id: "random-integer", label: "Random integer", origin: "browser", mode: "input", hint: "Inclusive min/max integer with unbiased rejection sampling." },
  { id: "random-decimal", label: "Random decimal", origin: "browser", mode: "input", hint: "Uniform decimal in a numeric range." },
  { id: "random-choice", label: "Random choice", origin: "browser", mode: "input", hint: "Pick one item from lines or comma-separated values." },
  { id: "image-editor", label: "Image editor", origin: "browser", mode: "image", hint: "Resize, rotate, flip, adjust color and export through Canvas." },
  { id: "password-generator", label: "Password generator", origin: "mixed", mode: "password", hint: "Browser random bytes + Rust password policy." },
  { id: "qr-generator", label: "QR Code generator", origin: "mixed", mode: "qr", hint: "Rust/WASM QR matrix plus Canvas style and export controls." },
  { id: "phone-code-identifier", label: "Phone code identifier", origin: "rust", mode: "phone", hint: "Rust lookup for Brazilian DDDs and international calling codes." },
  { id: "br-document-toolkit", label: "Brazilian documents", origin: "rust", mode: "selo", hint: "Local selo-style validation, formatting, generation, UF origin and PIX/person fixtures." },
  { id: "inspect-uuid-v7", label: "Inspect UUID v7", origin: "rust", mode: "input", hint: "Paste a UUIDv7 value." },
  { id: "inspect-ulid", label: "Inspect ULID", origin: "rust", mode: "input", hint: "Paste a 26-character ULID." },
  { id: "inspect-ksuid", label: "Inspect KSUID", origin: "rust", mode: "input", hint: "Paste a 27-character KSUID." },
  { id: "sha-256", label: "SHA-256", origin: "browser", mode: "input", hint: "Uses crypto.subtle.digest()." },
  { id: "sha-512", label: "SHA-512", origin: "browser", mode: "input", hint: "Uses crypto.subtle.digest()." },
  { id: "base64-encode", label: "Base64 encode", origin: "browser", mode: "input", hint: "Uses TextEncoder + btoa()." },
  { id: "base64-decode", label: "Base64 decode", origin: "browser", mode: "input", hint: "Uses atob() + TextDecoder." },
  { id: "url-encode", label: "URL encode", origin: "browser", mode: "input", hint: "Uses encodeURIComponent()." },
  { id: "url-decode", label: "URL decode", origin: "browser", mode: "input", hint: "Uses decodeURIComponent()." },
  { id: "hex-encode", label: "Hex encode", origin: "rust", mode: "input", hint: "UTF-8 text to hexadecimal." },
  { id: "hex-decode", label: "Hex decode", origin: "rust", mode: "input", hint: "Hexadecimal to UTF-8 text." },
  { id: "base32-encode", label: "Base32 encode", origin: "rust", mode: "input", hint: "RFC 4648 Base32 encode." },
  { id: "base32-decode", label: "Base32 decode", origin: "rust", mode: "input", hint: "RFC 4648 Base32 decode." },
  { id: "base58-encode", label: "Base58 encode", origin: "rust", mode: "input", hint: "Bitcoin-style Base58 encode." },
  { id: "base58-decode", label: "Base58 decode", origin: "rust", mode: "input", hint: "Bitcoin-style Base58 decode." },
  { id: "base64url-encode", label: "Base64url encode", origin: "rust", mode: "input", hint: "URL-safe Base64 without padding." },
  { id: "base64url-decode", label: "Base64url decode", origin: "rust", mode: "input", hint: "URL-safe Base64 to UTF-8 text." },
  { id: "crc32", label: "CRC32", origin: "rust", mode: "input", hint: "IEEE CRC-32 checksum." },
  { id: "fnv1a-32", label: "FNV-1a 32", origin: "rust", mode: "input", hint: "32-bit FNV-1a hash." },
  { id: "fnv1a-64", label: "FNV-1a 64", origin: "rust", mode: "input", hint: "64-bit FNV-1a hash." },
  { id: "rot13", label: "ROT13", origin: "rust", mode: "input", hint: "ASCII letter rotation." },
  { id: "caesar", label: "Caesar", origin: "rust", mode: "input", option: "shift", hint: "Set a numeric shift, default 13." },
  { id: "vigenere-encode", label: "Vigenere encode", origin: "rust", mode: "input", option: "key", hint: "Set an ASCII letter key." },
  { id: "vigenere-decode", label: "Vigenere decode", origin: "rust", mode: "input", option: "key", hint: "Set an ASCII letter key." },
  { id: "morse-encode", label: "Morse encode", origin: "rust", mode: "input", hint: "Text to Morse code." },
  { id: "morse-decode", label: "Morse decode", origin: "rust", mode: "input", hint: "Morse code to text." },
  { id: "leet-encode", label: "Leet encode", origin: "rust", mode: "input", hint: "Simple leetspeak transform." },
  { id: "leet-decode", label: "Leet decode", origin: "rust", mode: "input", hint: "Simple leetspeak decode." },
  { id: "atbash", label: "Atbash", origin: "rust", mode: "input", hint: "Reversible alphabet substitution cipher." },
  { id: "polybius-encode", label: "Polybius encode", origin: "rust", mode: "input", hint: "5x5 Polybius square, I/J merged." },
  { id: "polybius-decode", label: "Polybius decode", origin: "rust", mode: "input", hint: "Decode Polybius number pairs." }
].map((tool) => Object.freeze({
  ...tool,
  ...toolOptions[tool.id],
  sample: toolSamples[tool.id] || "",
  explain: toolExplanations[tool.id] || tool.hint
})));

export const toolGroups = Object.freeze([
  {
    id: "ids-time",
    label: "Identifiers & time",
    origin: "mixed",
    mode: "group",
    hint: "UUID, ULID, KSUID, Unix time conversion and timestamp inspectors.",
    tools: ["uuid-v4", "uuid-v7", "ulid", "ksuid", "time-convert", "inspect-uuid-v7", "inspect-ulid", "inspect-ksuid"]
  },
  {
    id: "random-values",
    label: "Random",
    origin: "browser",
    mode: "group",
    hint: "Browser CSPRNG bytes, numbers and picker utilities.",
    tools: ["random-bytes", "random-integer", "random-decimal", "random-choice"]
  },
  {
    id: "encoding-format",
    label: "Encoding & format",
    origin: "mixed",
    mode: "group",
    hint: "Browser-native and Rust text encoders/decoders.",
    tools: [
      "base64-encode", "base64-decode", "base64url-encode", "base64url-decode",
      "url-encode", "url-decode", "hex-encode", "hex-decode",
      "base32-encode", "base32-decode", "base58-encode", "base58-decode"
    ]
  },
  {
    id: "hashing-checksums",
    label: "Hashing & checksums",
    origin: "mixed",
    mode: "group",
    hint: "One-way hashes and accidental-corruption checksums.",
    tools: ["sha-256", "sha-512", "crc32", "fnv1a-32", "fnv1a-64"]
  },
  {
    id: "ciphers-text",
    label: "Ciphers & text",
    origin: "rust",
    mode: "group",
    hint: "Classic ciphers, text substitutions and Morse/Polybius transforms.",
    tools: [
      "rot13", "caesar", "vigenere-encode", "vigenere-decode",
      "morse-encode", "morse-decode", "leet-encode", "leet-decode",
      "atbash", "polybius-encode", "polybius-decode"
    ]
  },
  { id: "image-editor", label: "Image editor", origin: "browser", mode: "image", hint: "Resize, rotate, flip, adjust color and export through Canvas.", sourceTool: "image-editor" },
  { id: "password-generator", label: "Password generator", origin: "mixed", mode: "password", hint: "Browser random bytes + Rust password policy.", sourceTool: "password-generator" },
  { id: "qr-generator", label: "QR Code generator", origin: "mixed", mode: "qr", hint: "Rust/WASM QR matrix plus Canvas style and export controls.", sourceTool: "qr-generator" },
  { id: "phone-code-identifier", label: "Phone codes", origin: "rust", mode: "phone", hint: "Rust lookup for Brazilian DDDs and international calling codes.", sourceTool: "phone-code-identifier" },
  { id: "br-document-toolkit", label: "Brazilian documents", origin: "rust", mode: "selo", hint: "Local selo-style validation, formatting, generation, UF origin and PIX/person fixtures.", sourceTool: "br-document-toolkit" }
].map((group) => Object.freeze({
  ...group,
  tools: Object.freeze(group.tools || [group.sourceTool])
})));

export function findTool(id = "") {
  return tools.find((tool) => tool.id === id);
}

export function groupForTool(id = "") {
  return toolGroups.find((group) => group.id === id || group.sourceTool === id || group.tools.includes(id));
}

export async function runTool(id, input = "", option = "") {
  switch (id) {
    case "uuid-v4":
      return crypto.randomUUID();
    case "uuid-v7": {
      const wasm = await core();
      return wasm.uuid_v7(Date.now(), randomBytes(10));
    }
    case "ulid": {
      const wasm = await core();
      return wasm.ulid(Date.now(), randomBytes(10));
    }
    case "ksuid": {
      const wasm = await core();
      return wasm.ksuid(Math.floor(Date.now() / 1000), randomBytes(16));
    }
    case "time-convert":
      return convertTime(input);
    case "random-bytes":
      return randomBytesOutput(input, option);
    case "random-integer":
      return String(randomInteger(input));
    case "random-decimal":
      return randomDecimal(input, option);
    case "random-choice":
      return randomChoice(input);
    case "inspect-uuid-v7": {
      const wasm = await core();
      return formatInspection(wasm.inspect_uuid_v7(input));
    }
    case "inspect-ulid": {
      const wasm = await core();
      return formatInspection(wasm.inspect_ulid(input));
    }
    case "inspect-ksuid": {
      const wasm = await core();
      return formatInspection(wasm.inspect_ksuid(input));
    }
    case "sha-256":
      return digestHex("SHA-256", input);
    case "sha-512":
      return digestHex("SHA-512", input);
    case "base64-encode":
      return bytesToBase64(textEncoder.encode(input));
    case "base64-decode":
      return textDecoder.decode(base64ToBytes(input.trim()));
    case "url-encode":
      return encodeURIComponent(input);
    case "url-decode":
      return decodeURIComponent(input);
    case "hex-encode": {
      const wasm = await core();
      return wasm.hex_encode(input);
    }
    case "hex-decode": {
      const wasm = await core();
      return wasm.hex_decode(input);
    }
    case "base32-encode": {
      const wasm = await core();
      return wasm.base32_encode(input);
    }
    case "base32-decode": {
      const wasm = await core();
      return wasm.base32_decode(input);
    }
    case "base58-encode": {
      const wasm = await core();
      return wasm.base58_encode(input);
    }
    case "base58-decode": {
      const wasm = await core();
      return wasm.base58_decode(input);
    }
    case "base64url-encode": {
      const wasm = await core();
      return wasm.base64url_encode(input);
    }
    case "base64url-decode": {
      const wasm = await core();
      return wasm.base64url_decode(input);
    }
    case "crc32": {
      const wasm = await core();
      return wasm.crc32(input);
    }
    case "fnv1a-32": {
      const wasm = await core();
      return wasm.fnv1a32(input);
    }
    case "fnv1a-64": {
      const wasm = await core();
      return wasm.fnv1a64(input);
    }
    case "rot13": {
      const wasm = await core();
      return wasm.rot13(input);
    }
    case "caesar": {
      const wasm = await core();
      return wasm.caesar(input, Number.parseInt(option || "13", 10) || 0);
    }
    case "vigenere-encode": {
      const wasm = await core();
      return wasm.vigenere(input, option || "LEMON", false);
    }
    case "vigenere-decode": {
      const wasm = await core();
      return wasm.vigenere(input, option || "LEMON", true);
    }
    case "morse-encode": {
      const wasm = await core();
      return wasm.morse_encode(input);
    }
    case "morse-decode": {
      const wasm = await core();
      return wasm.morse_decode(input);
    }
    case "leet-encode": {
      const wasm = await core();
      return wasm.leet_encode(input);
    }
    case "leet-decode": {
      const wasm = await core();
      return wasm.leet_decode(input);
    }
    case "atbash": {
      const wasm = await core();
      return wasm.atbash(input);
    }
    case "polybius-encode": {
      const wasm = await core();
      return wasm.polybius_encode(input);
    }
    case "polybius-decode": {
      const wasm = await core();
      return wasm.polybius_decode(input);
    }
    default:
      throw new Error(`Unknown tool: ${id}`);
  }
}

export async function loadImage(file) {
  if (!file || !file.type.startsWith("image/")) {
    throw new Error("Choose an image file");
  }
  if (imageBitmap && imageBitmap.close) imageBitmap.close();
  try {
    imageBitmap = await createImageBitmap(file, { imageOrientation: "from-image" });
  } catch {
    imageBitmap = await createImageBitmap(file);
  }
  return {
    name: file.name,
    type: file.type || "image/*",
    bytes: file.size,
    width: imageBitmap.width,
    height: imageBitmap.height
  };
}

export function renderImage(canvas, settings = {}) {
  if (!imageBitmap) throw new Error("Choose an image file first");

  const width = positiveInt(settings.width, imageBitmap.width);
  const height = positiveInt(settings.height, imageBitmap.height);
  const rotate = Number(settings.rotate || 0) * Math.PI / 180;
  const quarterTurn = Math.abs(Number(settings.rotate || 0)) % 180 === 90;
  const drawWidth = quarterTurn ? height : width;
  const drawHeight = quarterTurn ? width : height;
  const context = canvas.getContext("2d", { alpha: true });

  canvas.width = width;
  canvas.height = height;
  if (settings.format === "image/jpeg") {
    context.fillStyle = "#ffffff";
    context.fillRect(0, 0, width, height);
  } else {
    context.clearRect(0, 0, width, height);
  }
  context.save();
  context.filter = [
    `brightness(${100 + clamp(Number(settings.brightness || 0), -100, 100)}%)`,
    `contrast(${clamp(Number(settings.contrast || 100), 0, 200)}%)`,
    `saturate(${clamp(Number(settings.saturation || 100), 0, 200)}%)`,
    `grayscale(${clamp(Number(settings.grayscale || 0), 0, 100)}%)`
  ].join(" ");
  context.translate(width / 2, height / 2);
  context.rotate(rotate);
  context.scale(settings.flip_horizontal ? -1 : 1, settings.flip_vertical ? -1 : 1);
  context.drawImage(imageBitmap, -drawWidth / 2, -drawHeight / 2, drawWidth, drawHeight);
  context.restore();

  return { width, height };
}

export function exportImage(canvas, type = "image/png", quality = 0.92) {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) resolve(blob);
      else reject(new Error(`Cannot export ${type}`));
    }, type, clamp(Number(quality || 0.92), 0.1, 1));
  });
}

export function imageFilename(sourceName = "image", type = "image/png") {
  const ext = type === "image/jpeg" ? "jpg" : type === "image/webp" ? "webp" : "png";
  const base = sourceName.replace(/\.[^.]+$/, "").replace(/[^\w.-]+/g, "-") || "image";
  return `${base}-edited.${ext}`;
}

export async function generatePassword(settings = {}) {
  const wasm = await core();
  const length = clampInt(settings.length, 1, 512, 16);
  const count = clampInt(settings.count, 1, 100, 1);
  let random = randomBytes(Math.max(1024, length * count * 8));
  for (let attempt = 0; attempt < 4; attempt += 1) {
    try {
      return wasm.password_generate(
        length,
        count,
        !!settings.lowercase,
        !!settings.uppercase,
        !!settings.numbers,
        !!settings.symbols,
        !!settings.exclude_similar,
        !!settings.exclude_ambiguous,
        !!settings.require_each,
        random
      );
    } catch (error) {
      if (!String(error).includes("not enough random bytes") || attempt === 3) throw error;
      random = randomBytes(random.length * 2);
    }
  }
  throw new Error("Password generation failed");
}

export async function renderQr(canvas, settings = {}) {
  const content = String(settings.content || "").trim();
  if (!content) throw new Error("QR content cannot be empty");

  const wasm = await core();
  const qr = JSON.parse(wasm.qr_matrix(content, settings.ecc || "Q"));
  const size = clampInt(settings.size, 96, 2048, 256);
  const padding = clampInt(settings.padding, 0, Math.floor(size / 2) - 1, 16);
  const dotColor = settings.color || "#000000";
  const bgColor = settings.background || "#ffffff";
  const moduleSize = (size - padding * 2) / qr.size;
  const context = canvas.getContext("2d", { alpha: settings.format !== "image/jpeg" });
  const customCorners = (settings.corner_square || "none") !== "none" || (settings.corner_dot || "none") !== "none";

  canvas.width = size;
  canvas.height = size;
  context.fillStyle = bgColor;
  context.fillRect(0, 0, size, size);

  for (let y = 0; y < qr.size; y += 1) {
    for (let x = 0; x < qr.size; x += 1) {
      if (qr.modules[y * qr.size + x] !== "1") continue;
      if (customCorners && inFinder(x, y, qr.size)) continue;
      drawModule(context, padding + x * moduleSize, padding + y * moduleSize, moduleSize, settings.dot || "square", dotColor);
    }
  }
  if (customCorners) {
    drawFinders(context, qr.size, padding, moduleSize, settings, bgColor, dotColor);
  }

  const logoError = await drawQrLogo(context, size, settings, bgColor);
  return {
    content,
    version_size: qr.size,
    canvas_size: size,
    padding,
    error_correction: qr.ecc,
    logo_error: logoError || undefined
  };
}

export function exportQr(canvas, type = "image/png", quality = 0.92) {
  return exportImage(canvas, type, quality);
}

export function qrFilename(type = "image/png") {
  const ext = type === "image/jpeg" ? "jpg" : type === "image/webp" ? "webp" : "png";
  return `qr-code.${ext}`;
}

export async function identifyPhone(input = "") {
  const wasm = await core();
  const lookup = JSON.parse(wasm.phone_lookup(input));
  lookup.results = lookup.results.map((result) => ({ ...result, flag: flagPath(result.iso) }));
  return lookup;
}

export async function formatPhoneInput(input = "") {
  const wasm = await core();
  return wasm.phone_format(input);
}

export async function runBrDocument(settings = {}) {
  const wasm = await core();
  const kind = String(settings.kind || "cpf");
  const action = String(settings.action || "validate");
  const input = String(settings.input || "");
  const uf = String(settings.uf || "SP");
  let random = randomBytes(4096);
  for (let attempt = 0; attempt < 4; attempt += 1) {
    try {
      return JSON.parse(wasm.br_document(kind, action, input, uf, random));
    } catch (error) {
      if (!String(error).includes("not enough random bytes") || attempt === 3) throw error;
      random = randomBytes(random.length * 2);
    }
  }
  throw new Error("Brazilian document tool failed");
}

export function flagPath(iso = "") {
  const code = String(iso).toLowerCase();
  return FLAG_ASSETS.has(code) ? `assets/flags/${code}.svg` : "assets/flags/world.svg";
}

async function core() {
  if (!corePromise) {
    corePromise = import("./pkg/core.js").then(async (mod) => {
      await mod.default();
      return mod;
    });
  }
  return corePromise;
}

function randomBytes(length) {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}

async function digestHex(algorithm, input) {
  const bytes = textEncoder.encode(input);
  const digest = await crypto.subtle.digest(algorithm, bytes);
  return hex(new Uint8Array(digest));
}

function formatInspection(json) {
  const data = JSON.parse(json);
  const lines = [
    `Kind: ${inspectionKindLabel(data.kind)}`,
    `ID: ${data.id}`
  ];
  if (data.timestamp_ms !== undefined) {
    const date = new Date(data.timestamp_ms);
    lines.push(`Timestamp: ${date.toISOString()}`);
    lines.push(`Unix milliseconds: ${data.timestamp_ms}`);
  }
  if (data.timestamp_secs !== undefined) {
    const date = new Date(data.timestamp_secs * 1000);
    lines.push(`Timestamp: ${date.toISOString()}`);
    lines.push(`Unix seconds: ${data.timestamp_secs}`);
  }
  if (data.random) lines.push(`Random payload: ${data.random}`);
  if (data.payload) lines.push(`Payload: ${data.payload}`);
  if (data.raw) lines.push(`Raw bytes: ${data.raw}`);
  return lines.join("\n");
}

function convertTime(input) {
  const trimmed = input.trim();
  const ms = trimmed ? parseTimeInput(trimmed) : Date.now();
  const date = new Date(ms);
  if (Number.isNaN(date.getTime())) {
    throw new Error("Invalid date or timestamp");
  }
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  return [
    `Input: ${trimmed || "now"}`,
    `UTC: ${date.toISOString()}`,
    `Local: ${new Intl.DateTimeFormat(undefined, { dateStyle: "full", timeStyle: "long" }).format(date)}`,
    `Unix seconds: ${Math.floor(ms / 1000)}`,
    `Unix milliseconds: ${ms}`,
    `Unix microseconds: ${Math.trunc(ms * 1000)}`,
    `Unix nanoseconds: ${Math.trunc(ms)}000000`,
    `Timezone: ${timezone}`
  ].join("\n");
}

function randomBytesOutput(input = "", option = "hex") {
  const length = clampInt(input || "16", 1, 4096, 16);
  const bytes = randomBytes(length);
  const format = String(option || "hex").trim().toLowerCase();
  if (format === "base64") return bytesToBase64(bytes);
  if (format === "base64url") return bytesToBase64(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  if (format === "decimal" || format === "dec") return [...bytes].join(" ");
  return hex(bytes);
}

function randomInteger(input = "") {
  const [min, max] = parseIntegerRange(input, 0, 100);
  const span = max - min + 1;
  if (span <= 0 || span > 0x100000000) {
    throw new Error("Integer range must contain 1 to 4294967296 values");
  }
  return min + randomU32Below(span);
}

function randomDecimal(input = "", option = "12") {
  const [min, max] = parseDecimalRange(input, 0, 1);
  const digits = clampInt(option || "12", 0, 20, 12);
  const value = min + randomUnit53() * (max - min);
  return Number.isInteger(value) ? String(value) : value.toFixed(digits).replace(/\.?0+$/, "");
}

function randomChoice(input = "") {
  const text = String(input || "").trim();
  const items = text.includes("\n")
    ? text.split(/\r?\n/)
    : text.split(",");
  const choices = items.map((item) => item.trim()).filter(Boolean);
  if (!choices.length) {
    throw new Error("Enter choices separated by new lines or commas");
  }
  return choices[randomU32Below(choices.length)];
}

function parseIntegerRange(input, fallbackMin, fallbackMax) {
  const values = String(input || "")
    .match(/-?\d+/g)
    ?.map((value) => Number.parseInt(value, 10))
    .filter(Number.isSafeInteger) || [];
  let min = values.length === 1 ? 0 : values[0] ?? fallbackMin;
  let max = values.length === 1 ? values[0] : values[1] ?? fallbackMax;
  if (min > max) [min, max] = [max, min];
  return [min, max];
}

function parseDecimalRange(input, fallbackMin, fallbackMax) {
  const values = String(input || "")
    .match(/-?\d+(?:\.\d+)?/g)
    ?.map(Number)
    .filter(Number.isFinite) || [];
  let min = values.length === 1 ? 0 : values[0] ?? fallbackMin;
  let max = values.length === 1 ? values[0] : values[1] ?? fallbackMax;
  if (min > max) [min, max] = [max, min];
  return [min, max];
}

function randomU32Below(limit) {
  const bucket = Math.floor(0x100000000 / limit) * limit;
  const view = new DataView(new ArrayBuffer(4));
  do {
    const bytes = randomBytes(4);
    bytes.forEach((byte, index) => view.setUint8(index, byte));
    const value = view.getUint32(0, false);
    if (value < bucket) return value % limit;
  } while (true);
}

function randomUnit53() {
  const bytes = randomBytes(7);
  let value = bytes[0] & 0x1f;
  for (let index = 1; index < bytes.length; index += 1) {
    value = value * 256 + bytes[index];
  }
  return value / 0x20000000000000;
}

function inspectionKindLabel(kind = "") {
  switch (kind) {
    case "uuid_v7": return "UUID v7";
    case "ulid": return "ULID";
    case "ksuid": return "KSUID";
    default: return String(kind || "identifier");
  }
}

function parseTimeInput(input) {
  if (/^-?\d+(\.\d+)?$/.test(input)) {
    const numeric = Number(input);
    const digits = input.replace(/^-/, "").replace(/\..*$/, "").length;
    if (digits >= 19) return Math.trunc(numeric / 1_000_000);
    if (digits >= 16) return Math.trunc(numeric / 1_000);
    if (digits >= 13) return Math.trunc(numeric);
    return Math.trunc(numeric * 1000);
  }
  const parsed = Date.parse(input);
  if (Number.isNaN(parsed)) {
    throw new Error("Invalid date or timestamp");
  }
  return parsed;
}

function bytesToBase64(bytes) {
  let binary = "";
  bytes.forEach((byte) => { binary += String.fromCharCode(byte); });
  return btoa(binary);
}

function base64ToBytes(base64) {
  const binary = atob(base64);
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

function hex(bytes) {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function positiveInt(value, fallback) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function clampInt(value, min, max, fallback) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.min(max, Math.max(min, parsed));
}

function inFinder(x, y, size) {
  return (x < 7 && y < 7) || (x >= size - 7 && y < 7) || (x < 7 && y >= size - 7);
}

function drawFinders(context, qrSize, padding, moduleSize, settings, bgColor, dotColor) {
  [[0, 0], [qrSize - 7, 0], [0, qrSize - 7]].forEach(([fx, fy]) => {
    const x = padding + fx * moduleSize;
    const y = padding + fy * moduleSize;
    const outerType = settings.corner_square === "none" ? "square" : settings.corner_square;
    const innerType = settings.corner_dot === "none" ? "square" : settings.corner_dot;
    drawModule(context, x, y, moduleSize * 7, outerType, settings.corner_square_color || dotColor);
    drawModule(context, x + moduleSize, y + moduleSize, moduleSize * 5, "square", bgColor);
    drawModule(context, x + moduleSize * 2, y + moduleSize * 2, moduleSize * 3, innerType, settings.corner_dot_color || dotColor);
  });
}

function drawModule(context, x, y, size, type, color) {
  context.fillStyle = color;
  const inset = type === "square" ? 0 : size * 0.08;
  const xx = x + inset;
  const yy = y + inset;
  const ss = size - inset * 2;
  if (type === "circle") {
    context.beginPath();
    context.arc(xx + ss / 2, yy + ss / 2, ss / 2, 0, Math.PI * 2);
    context.fill();
  } else if (type === "rounded" && context.roundRect) {
    context.beginPath();
    context.roundRect(xx, yy, ss, ss, Math.max(2, ss * 0.28));
    context.fill();
  } else {
    context.fillRect(xx, yy, ss, ss);
  }
}

async function drawQrLogo(context, size, settings, bgColor) {
  if (settings.image_type !== "url" || !settings.image_url) return "";
  try {
    const image = await loadHtmlImage(settings.image_url);
    const logoSize = Math.max(0, Math.min(size * 0.4, size * Number(settings.image_size || 20) / 100));
    if (!logoSize) return "";
    const margin = clampInt(settings.image_margin, 0, Math.floor(size / 4), 0);
    const x = (size - logoSize) / 2;
    const y = (size - logoSize) / 2;
    if (!settings.show_background_dots) {
      context.fillStyle = bgColor;
      context.fillRect(x - margin, y - margin, logoSize + margin * 2, logoSize + margin * 2);
    }
    context.drawImage(image, x, y, logoSize, logoSize);
    return "";
  } catch (error) {
    return error && error.message ? error.message : String(error);
  }
}

function loadHtmlImage(src) {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("Logo image could not be loaded"));
    image.src = src;
  });
}

const FLAG_ASSETS = new Set([
  "ar", "au", "bo", "br", "ca", "cl", "cn", "co", "de", "es", "fr", "gb",
  "in", "it", "jp", "kr", "mx", "nz", "pe", "pt", "py", "ru", "us", "uy", "ve"
]);
