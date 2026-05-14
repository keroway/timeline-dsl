# Timeline DSL Tutorial

## What is Timeline DSL?

Timeline DSL (`.tdsl`) is a domain-specific language for declaratively describing timeline data as text. It uses a C-style brace and semicolon syntax, making it well-suited for version control with Git and diff-based code review. The compiler parses `.tdsl` files and converts them into a JSON IR (intermediate representation). You can also automatically import open data from Wikidata via QIDs (entity identifiers), allowing you to build historical timelines quickly.

It is ideal for anyone who wants to work with structured data that has a time axis — history, culture, fictional world-building, and more. No knowledge of Rust or programming is required to read and write DSL files; all you need is a text editor. With the Wikidata integration feature, you can pull in base data for historical dynasties, people, and events with a single command.

---

## Installation

### One-line install (macOS / Linux)

```sh
curl -sSfL https://raw.githubusercontent.com/keroway/timeline-dsl/main/install.sh | sh
```

After installation, the `tdsl` command is added to your PATH.

### Install via cargo (for Rust developers)

```sh
cargo install --git https://github.com/keroway/timeline-dsl tdsl-cli
```

After installation, verify it works with:

```sh
tdsl --help
```

---

## Tutorial A: Building a Timeline Manually

This flow involves writing text directly to create a timeline for a fictional world or custom theme. No connection to Wikidata is required.

### A-1. Generate a template (tdsl init)

Use the `tdsl init` command to generate a skeleton `.tdsl` file for your timeline.

```bash
tdsl init \
  --output my_timeline.tdsl \
  --timeline "Fictional World Timeline" \
  --range-start 1000 \
  --range-end 1300 \
  --lanes "Kingdoms:kingdom,Events:incidents"
```

Option descriptions:

| Option | Description |
|---|---|
| `--output` | Output file path |
| `--timeline` | Title of the timeline |
| `--range-start` / `--range-end` | Display range (integer years; negative values are BCE) |
| `--lanes` | Lane definitions in `Label:ID` format, comma-separated |

Check the generated file:

```bash
cat my_timeline.tdsl
```

### A-2. Add items

Open `my_timeline.tdsl` in a text editor and append `span` / `event` / `event_range` entries.

```
timeline "Fictional World Timeline" {
    title "Fictional World Timeline";
    unit year;
    range 1000..1300;
    calendar proleptic_gregorian;
}

lane "Kingdoms" as kingdom { kind custom; order 10; }
lane "Events"   as incidents { kind custom; order 20; }

// A span covering the kingdom's existence
span kingdom 1001..1180 "Kingdom of Arcadia" {
    tags ["dynasty", "fictional"];
    id "span:arcadia";
};

// A point event
event incidents 1042 "Founding of the Dragon Knight Order" {
    tags ["founding", "fictional"];
    id "event:knights";
};

// A range event
event_range incidents 1175..1180 "War of the Black Mist" {
    tags ["war", "fictional"];
    id "range:black_mist";
};
```

How to choose between the three time element types:

| Type | Syntax | Use case |
|---|---|---|
| `span` | `span laneID start..end "label" {}` | A period of existence (a dynasty, a person's lifespan, etc.) |
| `event` | `event laneID year "label" {}` | Something that happened at a specific point in time |
| `event_range` | `event_range laneID start..end "label" {}` | Something that lasted for a period (a war, a disaster, etc.) |

### A-3. Quality check and auto-fix (tdsl lint --fix)

Use `tdsl lint` to detect issues such as undefined lane references, duplicate IDs, or `start > end`. Adding `--fix` applies safe automatic corrections.

```bash
# Check for issues only
tdsl lint my_timeline.tdsl

# Auto-fix
tdsl lint my_timeline.tdsl --fix
```

Examples of auto-fixes:

- Remove duplicate tags
- Remove empty tags
- Swap `start` and `end` when `start > end`
- Generate stable IDs for items without an `id` field

For CI integration using JSON output:

```bash
tdsl lint my_timeline.tdsl --format json
```

### A-4. Visualize as HTML (tdsl render)

Use `tdsl render` to generate a standalone HTML file. Open it in a browser to display the timeline.

```bash
tdsl render my_timeline.tdsl --output my_timeline.html
open my_timeline.html   # macOS
```

Use `--scale` to increase the scale for readability (default is 2):

```bash
tdsl render my_timeline.tdsl --scale 5 --output my_timeline.html
```

> The HTML file is a self-contained standalone file with no external dependencies. It consists only of inline SVG + CSS and has no JavaScript dependencies. Hovering over each element shows a tooltip with details such as label, duration, tags, and more.

---

## Tutorial B: Generating a Timeline from Wikidata

This flow semi-automatically builds timelines for historical dynasties, people, and organizations using Wikidata QIDs (entity identifiers). A network connection is required.

### B-1. Search for entities (tdsl search)

Search Wikidata using keywords for the subject you want to put on a timeline.

```bash
tdsl search "Han dynasty" --lang en -n 5
```

Example output:

```
Q7209  Han dynasty  Chinese imperial dynasty (206 BCE – 220 CE)
Q8733  Eastern Han  Successor dynasty to the Han (25–220 CE)
...
```

`-n` is the maximum number of results. `--lang` specifies the display language priority (comma-separated).

You can also resolve a QID from a Wikipedia URL:

```bash
tdsl resolve "https://en.wikipedia.org/wiki/Han_dynasty"
# -> Q7209
```

### B-2. Check suitability for timelines (tdsl inspect)

Verify that the QID you found has the necessary properties (founding year, dissolution year, etc.) for use in a timeline.

```bash
tdsl inspect Q7209 --lang en,ja
```

The output includes:

- Basic entity information (label and description)
- A list of properties that can be used in a timeline (P571 inception, P576 dissolved, P569 date of birth, etc.)
- A diagnostic result for timeline suitability

Commonly used properties:

| Property | Meaning | DSL expression |
|---|---|---|
| P569 | date of birth (person) | `claim(P569).year` |
| P570 | date of death (person) | `claim(P570).year` |
| P571 | inception (organization/dynasty) | `claim(P571).year` |
| P576 | dissolved/abolished | `claim(P576).year` |
| P580 | start time | `claim(P580).year` |
| P582 | end time | `claim(P582).year` |

### B-3. Generate a .tdsl scaffold (tdsl scaffold wikidata)

Automatically generate a `.tdsl` scaffold from a list of QIDs.

```bash
tdsl scaffold wikidata \
  --qids Q7183,Q7209 \
  --timeline "Chinese Dynasties (generated)" \
  --lang en,ja \
  --target auto \
  --lane-mode per-entity \
  --output china_scaffold.tdsl
```

Option descriptions:

| Option | Description |
|---|---|
| `--qids` | Target entity QIDs (comma-separated) |
| `--timeline` | Title of the timeline |
| `--lang` | Language priority for labels |
| `--target auto` | Auto-detect `span` / `event` |
| `--lane-mode per-entity` | Generate a lane per entity |
| `--output` | Output file path |

The generated `.tdsl` file contains `import` and `map` blocks:

```
import wikidata as wd {
    entity Q7183 as qin_dynasty;
    entity Q7209 as han_dynasty;
    policy merge_by_source;
}

map wd.qin_dynasty to span {
    lane qin;
    start claim(P571).year;
    end claim(P576).year;
    label label@en ?? label@ja;
    tags ["dynasty", "imported"];
}
```

### B-4. Syntax check (tdsl check)

Verify that the generated scaffold has no issues.

```bash
tdsl check china_scaffold.tdsl
```

If there are no errors, proceed to the next step. If errors are shown, refer to the line numbers in the error messages to fix the file.

> `tdsl check` only performs static parsing and semantic validation — it does not access Wikidata. Wikidata data is fetched when you run `tdsl build` or `tdsl render` without `--offline`.

### B-5. Visualize as HTML (tdsl render)

```bash
tdsl render china_scaffold.tdsl --output china_scaffold.html
open china_scaffold.html   # macOS
```

In environments without Wikidata access, add `--offline`:

```bash
tdsl render china_scaffold.tdsl --offline --output china_scaffold.html
```

---

## Frequently Asked Questions (FAQ)

### 1. How do I write BCE years?

Use negative integers. For example, 206 BCE is `-206`, and 221 BCE is `-221`.

```
span qin -221..-206 "Qin dynasty" { tags ["dynasty"]; };
```

Negative integers can also be used for the start of a `range`:

```
timeline "Ancient History" {
    range -500..500;
}
```

### 2. How do I add events not in Wikidata?

Write `event` / `event_range` / `span` directly in your `.tdsl` file.

```
event han -209 "Dazexiang Uprising" {
    tags ["revolt"];
    id "event:chen_sheng";
};
```

When appending to a file that already imports from Wikidata, you can add items using the same syntax.

### 3. What should I do when an error message appears?

Error messages show the line number and cause. Common patterns:

- **Undefined lane reference**: The lane ID used in a `span` / `event` does not exist in a `lane` declaration. Check the spelling of the lane ID.
- **start > end**: The start year is greater than the end year. Run `tdsl lint --fix` to auto-fix.
- **Wikidata fetch failure**: A network connectivity or rate-limit issue. `tdsl check` only performs static validation and does not access Wikidata. Use `tdsl build --offline` or `tdsl render --offline` to skip Wikidata access in those commands.
- **Parse error**: A semicolon or brace may be missing. Check the syntax around the line indicated in the error.

```bash
# Review error details
tdsl check my_timeline.tdsl

# Fix auto-correctable problems with lint --fix
tdsl lint my_timeline.tdsl --fix
```

### 4. How do I define multiple lanes?

Simply list multiple `lane` declarations. Use `order` to control the display order (lower numbers appear higher).

```
lane "Qin"       as qin    { kind dynasty; order 10; }
lane "Han"       as han    { kind dynasty; order 20; }
lane "Three Kingdoms" as sanguo { kind dynasty; order 30; }
```

`kind` is a classification label; you can freely specify values like `dynasty`, `person`, `nation`, etc. If you do not provide an explicit `as` ID, an ASCII slug is auto-generated from the label (for labels with only non-ASCII characters, names like `lane_1`, `lane_2`, ... are assigned automatically).

### 5. How do I use Timeline DSL without Wikidata access?

Add the `--offline` flag to `tdsl build` or `tdsl render` to skip Wikidata requests.

```bash
# Build offline (skip Wikidata fetch)
tdsl build my_timeline.tdsl --offline --pretty

# Render offline
tdsl render my_timeline.tdsl --offline --output out.html
```

> `tdsl check` always runs in static-only mode and never accesses Wikidata, so `--offline` is not applicable to it.

In offline mode, no data is imported via `import` / `map` blocks, so Wikidata-sourced items will not appear in the output. Statically defined items (`span` / `event` / `event_range`) are processed as usual.

### 6. How do I reset the cache?

The Wikidata fetch cache is stored under `~/.cache/tdsl/`. Use the `tdsl cache` command to manage it:

```bash
# Show cache status
tdsl cache status

# Clear all cache entries
tdsl cache clear

# Clear entries older than 30 days
tdsl cache clear --older-than 30
```

---

## Next Steps

- **DSL specification details**: [docs/dsl-spec.en.md](dsl-spec.en.md) — Grammar reference, all properties, and the complete IR structure specification
- **Sample files**:
  - `examples/china_dynasties.tdsl` — A simple sample with static definitions only
  - `examples/china_with_import.tdsl` — A sample with Wikidata integration
  - `examples/fictional_empire.tdsl` — A sample for fictional worlds
