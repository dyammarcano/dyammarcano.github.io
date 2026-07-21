# dyammarcano.github.io

Personal website of **Dyam Marcano** — Senior Backend Engineer based in São Paulo, Brazil.
14+ years building high-scale, fault-tolerant distributed systems for financial institutions.

Live at **[dyammarcano.github.io](https://dyammarcano.github.io/)**.

## Overview

A single-page, dependency-free static site. There is no build step and no framework —
just hand-written HTML with inline CSS, served directly by GitHub Pages.

## Structure

| File | Purpose |
|------|---------|
| `index.html` | The entire site — markup, inline styles, and a JSON-LD `schema.org` block for search/AI crawlers. |
| `llms.txt` | Machine-readable summary for AI assistants ([llmstxt.org](https://llmstxt.org/) convention). |
| `.nojekyll` | Disables Jekyll processing so files are served verbatim. |
| `README.md` | This file. |

## Development

Because the site is a plain static file, just open `index.html` in a browser,
or serve the folder locally:

```bash
python -m http.server 8000   # then visit http://localhost:8000
```

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
