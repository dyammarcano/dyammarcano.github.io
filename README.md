# dyammarcano.github.io

Personal website of **Dyam Marcano** — Senior Backend Engineer based in São Paulo, Brazil.
14+ years building high-scale, fault-tolerant distributed systems for financial institutions.

Live at **[dyammarcano.github.io](https://dyammarcano.github.io/)**.

## Overview

A static GitHub Pages site with a hand-written `index.html` and a small Rust/WebAssembly
tool core. The page still has no frontend framework or bundler; Rust code is compiled
with `wasm-pack --target web` and lazy-loaded by the Geek Tools modal.

## Structure

| File | Purpose |
|------|---------|
| `index.html` | The entire site — markup, inline styles, and a JSON-LD `schema.org` block for search/AI crawlers. |
| `core/` | Rust source for compact pure transforms used by the Geek Tools modal. |
| `wasm/geek.js` | Browser-side bridge: uses native Web APIs where available and lazy-loads the Rust/WASM core only when needed. |
| `wasm/pkg/` | Generated `wasm-pack` browser package committed for GitHub Pages deployment. |
| `assets/flags/` | Small local SVG flag assets used by the phone-code lookup, with a world fallback. |
| `llms.txt` | Machine-readable summary for AI assistants ([llmstxt.org](https://llmstxt.org/) convention). |
| `.nojekyll` | Disables Jekyll processing so files are served verbatim. |
| `README.md` | This file. |

## Development

Build the Rust/WASM package after changing `core/`:

```bash
wasm-pack build --target web --release --out-dir ../wasm/pkg --out-name core core
```

`wasm-pack 0.15.0` emits `wasm/pkg/.gitignore` with `*`; delete that generated file
before committing so GitHub Pages receives the package files.

Run Rust tests:

```bash
cd core
cargo test
```

Serve the folder locally. Use HTTP rather than `file://`, because browsers fetch
the `.wasm` module:

```bash
python -m http.server 8000   # then visit http://localhost:8000
```

Run the autonomous client-side UX test. It launches local Chrome/Edge headless,
opens the real page, exercises every Geek Tools entry on desktop and mobile
viewports, and writes screenshots to `.tmp/client-ux/`:

```bash
node scripts/client-ux-test.mjs
```

## Geek Tools routing

The modal groups the 45 leaf tools into category views such as Identifiers & time,
Random, Encoding & format, Hashing & checksums, Ciphers & text, image, password,
QR, phone, and Brazilian documents. Direct leaf routes still work; the UI opens the
right group and selects the requested operation.

It supports lightweight virtual routes through query parameters:

```text
/?geek=1
/?tool=ulid
/?tool=random-values&op=random-integer&input=1%2C100&run=1
/?tool=sha-256&input=hello&run=1
/?tool=encoding-format&op=base64-decode&input=aGVsbG8%3D&run=1
/?tool=image-editor
/?tool=password-generator
/?tool=qr-generator&input=https%3A%2F%2Fdyammarcano.github.io%2F
/?tool=phone-code-identifier&input=%2B55%2011
/?tool=br-document-toolkit&kind=cpf&action=validate&input=52998224725&run=1
/?tool=br-document-toolkit&kind=person&action=generate&uf=SP&run=1
/?tool=time-convert&input=1800000000&run=1
/?tool=inspect-uuid-v7&input=<uuid>&run=1
```

Tool state is not saved silently. Users can opt in with the `Remember` checkbox,
which stores the selected tool, input, option, and output in `localStorage`.

The image editor follows the "do not reimplement browser primitives" boundary:
file loading, resize, rotation, flipping, color filters, preview, and PNG/JPEG/WebP
export use browser Canvas APIs. Add a separate lazy-loaded WASM imaging chunk
later only for heavier operations that Canvas does not cover well.

The password generator uses browser `crypto.getRandomValues()` for entropy, then
Rust/WASM applies the character-set policy, exclusions, required sets, and shuffle.
The QR generator uses the small MIT-licensed `qrcodegen` Rust crate for
standards-compliant QR module generation, then Canvas handles styling and
PNG/JPEG/WebP export.

Phone DDD/country-code lookup data and matching live in Rust/WASM. Small local SVG
flags remain in `assets/flags/` and are only attached by the browser renderer.

The Brazilian documents tool implements a compact Rust/WASM port of the selo surface:
CPF, alphanumeric CNPJ, CNH, PIS/PASEP/NIS, RENAVAM, Titulo Eleitoral, CEP,
Brazilian phone, license plate, CNS, RG-SP, Inscricao Estadual for SP/MG/RS/PR,
PIX keys, auto-detection, and synthetic person fixtures.

Additional compact Rust/WASM transforms include Hex, Base32, Base58, Base64url,
CRC32, FNV-1a 32/64, Morse, Leet, Atbash, and Polybius.

## Deployment

GitHub Pages serves the `master` branch automatically. Any push to `master`
redeploys the live site within about a minute — no CI or build required.

## Keeping the AI summaries in sync

Self-reported facts (years of experience, employers, tech stack) appear in
**three** places that must be kept consistent when edited:

- the `<meta>` description and `og:description` tags in `index.html`,
- the JSON-LD `schema.org` block in `index.html`,
- the visible page copy in `index.html` and the summary in `llms.txt`.

## License

Content © Dyam Marcano. All rights reserved.
