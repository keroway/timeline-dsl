# Timeline DSL Language Specification

## Overview

Timeline DSL (`.tdsl`) is a domain-specific language for declaratively describing timeline data. It uses a C-style brace + semicolon syntax, prioritizing readability and ease of Git diff management.

## Grammar (EBNF)

```ebnf
<document>     ::= { <statement> }

<statement>    ::= <timeline>
                 | <lane>
                 | <span>
                 | <event>
                 | <event_range>
                 | <import_block>
                 | <map_block>
                 | <template_block>
                 | <apply_block>

<timeline>     ::= "timeline" <string> "{" { <timeline_setting> } "}"
<timeline_setting>
               ::= "title" <string> ";"
                 | "unit" <identifier> ";"
                 | "range" <number> ".." <number> ";"
                 | "calendar" <identifier> ";"
                 | "color_map" "{" { <identifier> ":" <string> ";" } "}"

<lane>         ::= "lane" <string> ["as" <identifier>] "{" { <lane_prop> } "}"
<lane_prop>    ::= "kind" <identifier> ";"
                 | "order" <number> ";"

<span>         ::= "span" <identifier> <number> ".." <number> <string>
                   <block_options> ";"
<event>        ::= "event" <identifier> <number> <string>
                   <block_options> ";"
<event_range>  ::= "event_range" <identifier> <number> ".." <number> <string>
                   <block_options> ";"

<block_options> ::= "{" { <option> } "}"
<option>       ::= "tags" "[" <string_list> "]" ";"
                 | "source" <source_ref> ";"
                 | "origin" <identifier> ";"
                 | "id" <string> ";"

<import_block> ::= "import" <source_name> ["as" <identifier>]
                   "{" { <import_stmt> } "}"
<import_stmt>  ::= "entity" <qid> ["as" <identifier>] ";"
                 | "query" <string> ["as" <identifier>] ";"
                 | "policy" <policy_name> ";"
                 | "policy" "field_priority" "{" { <field_strategy> } "}"
<field_strategy> ::= ("label" | "time" | "tags") ":" ("manual" | "wikidata" | "merge") ";"

<map_block>    ::= "map" <import_ref> "to" <mapping_target>
                   "{" { <mapping_rule> } "}"
<mapping_target> ::= "span" | "event" | "event_range"
<mapping_rule> ::= "lane" <identifier> ";"
                 | "start" <expr> ";"
                 | "end" <expr> ";"
                 | "time" <expr> ";"
                 | "label" <expr> ";"
                 | "tags" "[" <string_list> "]" ";"

<template_block> ::= "template" <string> ["as" <identifier>]
                   "to" <mapping_target> "{" { <mapping_rule> } "}"

<apply_block>  ::= "apply" <identifier> "to" <identifier>
                   "{" { <apply_override> } "}"
<apply_override> ::= "lane" <identifier> ";"

<expr>         ::= <claim_expr> | <lang_expr> | <literal>
<claim_expr>   ::= "claim(" <property_id> ")" ["." <function>]
<lang_expr>    ::= "label@" <lang_code> ["??" <lang_expr>]

<source_ref>   ::= <identifier> ":" <qid>
<string_list>  ::= <string> { "," <string> }
<qid>          ::= "Q" <digits>
<property_id>  ::= "P" <digits>
<identifier>   ::= /[A-Za-z_][A-Za-z0-9_-]*/
<number>       ::= /"-"? [0-9]+/
```

## Syntax Element Details

### timeline

Defines the global metadata for the timeline. One per file.

```
timeline "Chinese Dynasties" {
    title "Chinese Dynasties";
    unit year;
    range -500..2000;
    calendar proleptic_gregorian;
    color_map {
        dynasty: "#3366cc";
        war:     "#cc0000";
    }
}
```

| Property | Required | Description |
|---|---|---|
| `title` | Optional | Display title of the timeline |
| `unit` | Optional | Time unit (`year`) |
| `range` | Optional | Display range in `start..end` format. Negative values are BCE |
| `calendar` | Optional | Calendar system (e.g., `proleptic_gregorian`) |
| `color_map` | Optional | Tag-to-color mapping. Define multiple entries as `tag_name: "#hex_color_code";` |

Colors defined in `color_map` are automatically applied during `tdsl render`. They can be overridden with the `--color-map "war=#cc0000"` CLI flag.

### lane

Defines a vertical category (lane) for the timeline. Used to represent dynasties, people, nations, organizations, and so on.

```
lane "Han" as han { kind dynasty; order 20; }
```

| Property | Required | Description |
|---|---|---|
| `as <id>` | Optional | Internal identifier. Auto-generated as a slug from the label if omitted |
| `kind` | Optional | Classification (`dynasty`, `person`, `nation`, etc.) |
| `order` | Optional | Initial display order (integer) |

### span

Represents a period of existence, such as a dynasty's reign or a person's lifespan.

```
span han -206..220 "Han" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };
```

- 1st argument: lane ID
- 2nd argument: `start..end` (integer range)
- 3rd argument: label (string)

### event

Represents an event at a specific point in time.

```
event han -209 "Dazexiang Uprising" {};
```

- 1st argument: lane ID
- 2nd argument: point in time (integer)
- 3rd argument: label (string)

### event_range

Represents an event spanning a period of time, such as a war, disaster, or project.

```
event_range han 184..204 "Yellow Turban Rebellion" { tags ["war"]; };
```

- 1st argument: lane ID
- 2nd argument: `start..end` (integer range)
- 3rd argument: label (string)

### block_options (common options)

Options that can be attached to `span`, `event`, and `event_range`.

| Option | Description | Example |
|---|---|---|
| `tags` | List of tags | `tags ["war", "major"];` |
| `source` | Data source (Wikidata, etc.) | `source wd:Q7209;` |
| `id` | Stable element identifier | `id "span:han";` |
| `origin` | Origin identifier | `origin imported;` |

### import

Declares an import of data from an external source.

```
import wikidata as wd {
    entity Q7183 as qin_dynasty;
    entity Q7209 as han_dynasty;
    query "SELECT ?item WHERE { ... }" as samurai;
    policy merge_by_source;
    policy field_priority {
        label: manual;    // prefer manually defined labels
        time:  wikidata;  // prefer Wikidata for time values
        tags:  merge;     // merge tags from both sources
    }
}
```

| Element | Description |
|---|---|
| `entity <QID>` | Specify a particular Wikidata entity |
| `query <SPARQL>` | Retrieve multiple entities via a SPARQL query |
| `policy <name>` | Merge strategy on re-import |
| `policy field_priority { ... }` | Per-field merge strategy |
| `as <alias>` | Alias for the import block or entity |

#### Re-import policies

| Policy | Behavior |
|---|---|
| `merge_by_source` | Treats ID conflicts as errors (default) |
| `overwrite_imported` | Overwrites only existing imported items; conflicts with manually defined items are errors |
| `keep_manual` | Skips the incoming import item and keeps the existing item on ID conflict |

#### Field priority policy (field_priority)

Provides finer control than whole-block policies like `merge_by_source` — lets you specify a merge strategy per field.

| Field | Value | Behavior |
|---|---|---|
| `label` / `time` / `tags` | `manual` | Keep the existing manually defined value (ignore Wikidata) |
| `label` / `time` / `tags` | `wikidata` | Prefer the Wikidata value (overwrite the manual value) |
| `label` / `time` / `tags` | `merge` | Keep both (union for `tags`; Wikidata value adopted for `label`/`time`) |

### map

Defines rules for converting imported entities into timeline items.

```
map wd.han_dynasty to span {
    lane han;
    start claim(P571).year;
    end claim(P576).year;
    label label@en ?? label@ja;
    tags ["dynasty", "imported"];
}
```

> `source` is automatically assigned to imported items as `wd:<entity_id>`. Explicit specification inside a `map` block is deprecated.

| Property | Description |
|---|---|
| `lane` | ID of the target lane |
| `start` | Expression to compute the start time |
| `end` | Expression to compute the end time |
| `time` | Expression to compute the point in time (for `event`) |
| `label` | Expression to compute the label |
| `tags` | List of tags |

### Expressions

#### claim expression

Retrieves a property value from Wikidata.

```
claim(P571).year    // Convert the time value of P571 (inception) to a year integer
claim(P569).year    // Convert the time value of P569 (date of birth) to a year integer
```

#### label expression

Retrieves the label of a Wikidata entity with language fallback.

```
label@en             // English label
label@en ?? label@ja // Fall back to Japanese if no English label exists
```

## Comments

Both line comments and block comments are supported.

```
// This is a line comment

/* This is
   a block comment */
```

## Wikidata Property Reference

### People

| Property | Meaning | Usage |
|---|---|---|
| P569 | date of birth | Birth year: `claim(P569).year` |
| P570 | date of death | Death year: `claim(P570).year` |
| P39 | position held | Office held; use P580/P582 for duration |

### Organizations, Nations, and Dynasties

| Property | Meaning | Usage |
|---|---|---|
| P571 | inception | Founding year: `claim(P571).year` |
| P576 | dissolved/abolished | Dissolution year: `claim(P576).year` |

### Time

| Property | Meaning | Usage |
|---|---|---|
| P580 | start time | Start of a period |
| P582 | end time | End of a period |
| P585 | point in time | A specific point-in-time event |

## Representing Time

- Positive integers: CE year (e.g., `220` = 220 CE)
- Negative integers: BCE year (e.g., `-206` = 206 BCE)
- Range: `start..end` (e.g., `-206..220`)
- Wikidata time values are converted to integer years using the `.year` function

## CLI

### Subcommand Reference

| Command | Purpose |
|---|---|
| `tdsl build <file>` | Convert `.tdsl` to JSON IR |
| `tdsl check <file>` | Syntax and semantic check |
| `tdsl ast <file>` | Dump the AST |
| `tdsl render <file>` | Generate HTML / SVG (`--format html\|svg`, `--interactive`) |
| `tdsl decompile <json>` | Convert JSON IR back to `.tdsl` source |
| `tdsl fetch <QID>` | Inspect a Wikidata entity |
| `tdsl search <query>` | Search Wikidata for candidates |
| `tdsl inspect <QID>` | Diagnose timeline suitability |
| `tdsl resolve <wikipedia-url>` | Resolve a Wikipedia URL to a QID |
| `tdsl scaffold wikidata ...` | Generate a `.tdsl` scaffold from a set of QIDs |
| `tdsl init ...` | Generate a `.tdsl` template for manual authoring |
| `tdsl import-csv <csv>` | Generate `span/event/event_range` from CSV |
| `tdsl lint <file> [--fix]` | Quality check with safe auto-correction |
| `tdsl cache status` | Show local cache status |
| `tdsl cache clear [--older-than <days>]` | Delete cache entries |

### Quickstart flow (Wikidata-based)

```bash
tdsl search "Han dynasty" --lang en -n 5
tdsl resolve "https://en.wikipedia.org/wiki/Han_dynasty"
tdsl inspect Q7209 --lang en,ja
tdsl scaffold wikidata --qids Q7183,Q7209 --timeline "Chinese Dynasties (generated)" --lang en,ja --target auto --lane-mode per-entity --output /tmp/china_scaffold.tdsl
tdsl render /tmp/china_scaffold.tdsl --output /tmp/china_scaffold.html
```

> `search / inspect / resolve / scaffold wikidata` require a network connection.

### Quickstart flow (manual authoring)

```bash
tdsl init --output /tmp/manual.tdsl --timeline "Fictional World Timeline" --range-start 1000 --range-end 1300 --lanes "Kingdoms:kingdom,Events:incidents"
tdsl import-csv examples/fictional_empire_items.csv --append /tmp/manual.tdsl
tdsl lint /tmp/manual.tdsl --fix
tdsl render /tmp/manual.tdsl --output /tmp/manual.html
```

### `tdsl render`

`tdsl render` visualizes the timeline as a standalone HTML file rather than outputting JSON IR.

```bash
tdsl render input.tdsl --output timeline.html [--scale N] [--offline]
```

| Option | Description |
|---|---|
| `--output` | Output path; defaults to stdout if omitted |
| `--scale` | Pixel width per year (default: 2) |
| `--offline` | Skip Wikidata fetch |

### Output specification

- **Format**: Single HTML file with inline SVG + CSS. No JavaScript dependencies.
- **Layout**:
  - Horizontal axis: time (uses `timeline.range`)
  - Vertical axis: lanes stacked top-to-bottom in ascending `order`
  - Axis tick intervals are chosen automatically based on the range (10 / 20 / 50 / 100 years, etc.)
- **Element rendering**:
  - `span` → rounded rectangle (centered in the lane band)
  - `event_range` → narrower rectangle (lower part of the lane band)
  - `event` → vertical line + small circle marker
- **Tooltips**: Hovering over each element shows a `<title>` element with the label, duration, tags, source, and ID.

### Constraints (MVP)

- No zoom, pan, or search (no JS)
- Tag-to-color map not applied (`span` is blue, `event_range` is red — fixed colors)
- No raster output such as PNG/PDF (use browser print or screenshot as an alternative)

## License and Data Usage

- **Wikidata structured data**: CC0 license. Free to use without attribution.
- **Wikipedia text and figures**: CC BY-SA 4.0. Attribution and same-license application required when quoting.

## Future Extensions (Not Yet Implemented)

### template / apply

A syntax for templating mapping rules and applying them to multiple entities.

```
template PersonLife(entity) {
    lane entity.label@en ?? entity.label@ja as entity.qid;
    let birth = entity.claim(P569).year;
    let death = entity.claim(P570).year;
    if birth != null && death != null {
        span entity.qid birth..death "Life" {
            tags ["person"];
            source wd:entity.qid;
        };
    }
}

apply PersonLife to wd.SengokuSamurai;
```

### Field-level priority

Use `policy field_priority { ... }` to specify per-field strategies on re-import:

```
import wikidata as wd {
    entity Q7209 as han_dynasty;
    policy field_priority {
        label: manual;    // keep manually edited labels
        time: wikidata;   // overwrite with the latest Wikidata values
        tags: merge;      // combine tags from both sources
    }
}
```

| Field | Strategy | Effect |
|---|---|---|
| `label` | `manual` | Keep the existing label |
| `label` | `wikidata` | Overwrite with the Wikidata label |
| `time` | `manual` | Keep the existing start/end/time |
| `time` | `wikidata` | Overwrite with Wikidata time values |
| `tags` | `manual` | Keep existing tags |
| `tags` | `wikidata` | Overwrite with Wikidata tags |
| `tags` | `merge` | Combine tags from both sources (no duplicates) |

All fields have default values, so you can specify only a subset:
- `label`: default `manual`
- `time`: default `wikidata`
- `tags`: default `merge`
