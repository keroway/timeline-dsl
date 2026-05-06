# Timeline DSL — VS Code Extension

Syntax highlighting for **Timeline DSL** (`.tdsl`) files.

Timeline DSL is a domain-specific language for building historical timelines with [Wikidata](https://www.wikidata.org/) integration.

- **Landing page**: https://timeline-dsl-lp.pages.dev/
- **Try online (WebUI)**: https://keroway.github.io/timeline-dsl/
- **GitHub**: https://github.com/keroway/timeline-dsl

---

## Features

- Syntax highlighting for all Timeline DSL constructs
- Keywords: `timeline`, `lane`, `span`, `event`, `event_range`, `import`, `map`
- String literals, comments (`//` and `/* */`)
- Wikidata entity IDs (`Q123`), property IDs (`P569`), and references (`wd:Q123`)
- Wikidata expressions: `claim(P571).year`, `label@ja`
- Numeric literals including negative years (e.g. `-221` for 221 BCE)

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
import {
    wd:Q7209 as han_entity;
}
map han_entity -> han {
    target_type span;
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
