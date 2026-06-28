# Timeline DSL — VS Code Extension

Syntax highlighting and language intelligence for **Timeline DSL** (`.tdsl`) files.

Timeline DSL is a domain-specific language for building historical timelines with [Wikidata](https://www.wikidata.org/) integration.

- **Landing page**: <https://timeline-dsl-lp.pages.dev/>
- **Try online (WebUI)**: <https://keroway.github.io/timeline-dsl/>
- **GitHub**: <https://github.com/keroway/timeline-dsl>

---

## Features

### Syntax Highlighting

- Keywords: `timeline`, `lane`, `group`, `span`, `event`, `event_range`, `import`, `map`, `template`, `apply`, `color_map`
- String literals, comments (`//` and `/* */`)
- Wikidata entity IDs (`Q123`), property IDs (`P569`), and references (`wd:Q123`)
- Wikidata expressions: `claim(P571).year`, `label@ja`
- Numeric literals including negative years (e.g. `-221` for 221 BCE)

### Language Server (LSP) — requires `tdsl` binary

When the `tdsl` CLI is installed, this extension automatically starts the LSP server (`tdsl lsp`) and provides:

- **Diagnostics** — error and warning highlighting, updated on every edit
- **Completion** — context-aware keyword and snippet suggestions (e.g. `claim(P123)`, `label@ja` inside `map {}`)
- **Hover** — lane label/kind/order on hover; QID hover shows cached entity info (cache is populated by `tdsl build` or `tdsl render`; shows a hint if not cached)
- **Go to Definition** — jump to lane declarations
- **Find References** — find all usages of a lane ID
- **Rename** — rename a lane and all its references (only lanes declared with an explicit `as <alias>`; auto-slug lanes are not renameable)
- **Code Actions** — quick fixes from `tdsl lint`
- **Document Symbols** — outline view and breadcrumb navigation
- **Formatting** — format the document (comments are preserved like `tdsl fmt`; comments inside blocks may be relocated to canonical positions)

#### Installing the `tdsl` binary

```bash
# Homebrew (macOS / Linux)
brew tap keroway/tap
brew install tdsl

# Cargo
cargo install --git https://github.com/keroway/timeline-dsl tdsl-cli
```

See the [installation guide](https://github.com/keroway/timeline-dsl#installation) for other platforms and options.

#### Configuration

| Setting | Default | Description |
|---|---|---|
| `timelineDsl.serverPath` | `""` | Path to the `tdsl` binary. If empty, the binary is resolved from `PATH`. |

## Example

```tdsl
// Chinese Dynasties Timeline

timeline "Chinese Dynasties" {
    unit year;
    range -500..2000;
    calendar proleptic_gregorian;
}

lane "Qin" as qin { kind dynasty; order 10; }
lane "Han" as han { kind dynasty; order 20; }

span qin -221..-206 "Qin Dynasty" { source wd:Q7183; };
span han -206..220  "Han Dynasty" { source wd:Q7209; };

event qin -221 "Unification of China" {};

// Import from Wikidata
import wikidata as wd {
    entity Q7209 as han_entity;
}
map wd.han_entity to span {
    lane han;
    label label@en;
    start claim(P571).year;
    end   claim(P576).year;
}
```

## Usage

Install the [Timeline DSL CLI](https://github.com/keroway/timeline-dsl) to compile `.tdsl` files:

```bash
# Build to JSON IR
tdsl build my_timeline.tdsl --pretty

# Render to HTML/SVG
tdsl render my_timeline.tdsl -o output.html

# Check for errors
tdsl check my_timeline.tdsl
```

## Links

- [GitHub Repository](https://github.com/keroway/timeline-dsl)
- [Landing Page](https://timeline-dsl-lp.pages.dev/)
- [WebUI](https://keroway.github.io/timeline-dsl/)
- [DSL Specification](https://github.com/keroway/timeline-dsl/blob/main/docs/dsl-spec.md)
- [Report an Issue](https://github.com/keroway/timeline-dsl/issues)

## License

MIT © keroway
