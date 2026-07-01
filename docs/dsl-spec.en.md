# Timeline DSL Language Specification

## Overview

Timeline DSL (`.tdsl`) is a domain-specific language for declaratively describing timeline data. It uses a C-style brace + semicolon syntax, prioritizing readability and ease of Git diff management.

> For detailed design of month/day precision (`YYYY-MM` / `YYYY-MM-DD` format), see [spec-date-precision.md](spec-date-precision.md).

## Grammar (EBNF)

```ebnf
<document>     ::= { <statement> }

<statement>    ::= <timeline>
                 | <lane>
                 | <group>
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
                 | "range" <time_value> ".." <time_value> ";"
                 | "calendar" <identifier> ";"
                 | "color_map" "{" { <identifier> ":" <string> ";" } "}"

<lane>         ::= "lane" <string> ["as" <identifier>] "{" { <lane_prop> } "}"
<lane_prop>    ::= "kind" <identifier> ";"
                 | "order" <number> ";"

<group>        ::= "group" <string> "{" <lane> { <lane> } "}"

<span>         ::= "span" <identifier> <time_value> ".." <time_value> <string>
                   <block_options> ";"
<event>        ::= "event" <identifier> <time_value> <string>
                   <block_options> ";"
<event_range>  ::= "event_range" <identifier> <time_value> ".." <time_value> <string>
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
                 | "start" <map_expr> ";"
                 | "end" <map_expr> ";"
                 | "time" <map_expr> ";"
                 | "label" <lang_expr> ";"
                 | "tags" "[" <string_list> "]" ";"
                 | "filter" <filter_expr> ";"
                 | "expand" "claim(" <property_id> ")" ";"
<filter_expr>  ::= <filter_or>
<filter_or>    ::= <filter_and> { "||" <filter_and> }
<filter_and>   ::= <filter_not> { "&&" <filter_not> }
<filter_not>   ::= ["!"] <filter_atom>
<filter_atom>  ::= "(" <filter_expr> ")"
                 | <label_ref> <string_match_op> <string>
                 | <filter_operand> <compare_op> <filter_operand>
<string_match_op> ::= "contains" | "startswith"
<compare_op>   ::= ">=" | "<=" | "==" | "!=" | ">" | "<"
<filter_operand> ::= "null" | <claim_expr> | <number>

<template_block> ::= "template" <string> ["as" <identifier>]
                   "to" <mapping_target> "{" { <mapping_rule> } "}"

<apply_block>  ::= "apply" <identifier> "to" <identifier>
                   "{" { <apply_override> } "}"
<apply_override> ::= "lane" <identifier> ";"

<claim_expr>   ::= "claim(" <property_id> ")" ["." "qualifier(" <property_id> ")"] ["." <function>] [<claim_offset>]
<map_expr>     ::= (<claim_expr> | <number>) { "??" (<claim_expr> | <number>) }
<lang_expr>    ::= <label_ref> { "??" <label_ref> }
<label_ref>    ::= "label@" <lang_code>

<source_ref>   ::= <identifier> ":" <qid>
<string_list>  ::= <string> { "," <string> }
<qid>          ::= "Q" <digits>
<property_id>  ::= "P" <digits>
<identifier>   ::= /[A-Za-z_][A-Za-z0-9_-]*/
<number>       ::= /"-"? [0-9]+/
<time_value>   ::= <date_time> | <date> | <year_month> | <year>
<year>         ::= /"-"? [0-9]+/
<year_month>   ::= /"-"? [0-9]{1,4} "-" [0-9]{2}/
<date>         ::= /"-"? [0-9]{1,4} "-" [0-9]{2} "-" [0-9]{2}/
<date_time>    ::= /"-"? [0-9]{1,4} "-" [0-9]{2} "-" [0-9]{2} "T" [0-9]{2} ":" [0-9]{2}/
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

Colors defined in `color_map` are automatically applied during `tdsl render`. `color_map` accepts hex colors (`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`) and simple CSS named color keywords. More complex CSS values are intentionally ignored by the renderer; use CLI `--custom-css` for advanced styling. Values can be overridden with the `--color-map "war=#cc0000"` CLI flag.

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

### group

Defines a group that bundles multiple lanes into a visual hierarchy. When rendered, a group label and group boundary lines are displayed.

```
group "Ancient" {
    lane "Qin" as qin { kind dynasty; order 10; }
    lane "Han" as han { kind dynasty; order 20; }
}
```

- A group must contain one or more `lane` declarations
- Lanes inside a group carry a `group` field (the group name) in the IR. The field is omitted for lanes that do not use `group`
- Existing `.tdsl` files that do not use `group` keep working as before (backward compatible)

### span

Represents a period of existence, such as a dynasty's reign or a person's lifespan.

```
span han -206..220 "Han" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };

// Month/day precision example
span ww2 1939-09-01..1945-09-02 "World War II" { tags ["war"]; };
```

- 1st argument: lane ID
- 2nd argument: `start..end` (time value range; month/day precision is also accepted, e.g. `1939-09-01..1945-09-02`)
- 3rd argument: label (string)

### event

Represents an event at a specific point in time.

```
event han -209 "Dazexiang Uprising" {};
```

- 1st argument: lane ID
- 2nd argument: point in time (time value; month/day precision is also accepted, e.g. `1969-07-20`)
- 3rd argument: label (string)

### event_range

Represents an event spanning a period of time, such as a war, disaster, or project.

```
event_range han 184..204 "Yellow Turban Rebellion" { tags ["war"]; };
```

- 1st argument: lane ID
- 2nd argument: `start..end` (time value range)
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

The `<target_type>` in `map <alias> to <target_type> { ... }` must be **one of `span` / `event` / `event_range`**. Any other value (e.g., `timeline` or `item`) causes the parse error `Unknown map target type '<value>' (expected one of: span, event, event_range)` (see [E004 in the error catalog](./error-catalog.md#e004-不明な-map-ターゲット型)).

| target_type | Generated item kind | Required time properties |
|---|---|---|
| `span` | Period (start to end) | `start` / `end` |
| `event` | Point event | `time` |
| `event_range` | Range event | `start` / `end` |

> `source` is automatically assigned to imported items as `wd:<entity_id>`. Explicit specification inside a `map` block is deprecated.

| Property | Description |
|---|---|
| `lane` | ID of the target lane |
| `start` | Expression to compute the start time |
| `end` | Expression to compute the end time |
| `time` | Expression to compute the point in time (for `event`) |
| `label` | Expression to compute the label |
| `tags` | List of tags |
| `filter` | Condition expression to filter entities (multiple `filter` rules are all evaluated as AND) |

#### filter expressions

Use `filter` rules to narrow down entities. Multiple `filter` rules are all evaluated as AND.

**Numeric comparison** (`>=`, `<=`, `==`, `!=`, `>`, `<`):

```
filter claim(P580).year > 1000;
filter claim(P576).year != null;
```

**String matching** (`contains` / `startswith`):

```
filter label@en contains "dynasty";       // only entities whose label contains "dynasty"
filter label@en startswith "Han";         // only entities whose label starts with "Han"
filter !(label@en contains "candidate");  // only entities whose label does not contain "candidate"
```

`label@<lang>` accepts any language code. Entities that have no label in the specified language evaluate to `false` (excluded) — there is no silent fallback.

**Logical operators** (`&&`, `||`, `!`, parentheses):

```
filter label@en contains "dynasty" && claim(P580).year > 0;
filter claim(P580).year > 500 || claim(P571).year > 500;
```

### template / apply

Define a reusable mapping pattern with `template`, and apply it to multiple imports with `apply`.

```
// Template definition
template "Dynasty span" as dynasty_span
    to span {
        start claim(P571).year;
        end claim(P576).year;
        label label@en ?? label@ja;
    }

// Apply the template (only `lane` can be overridden on the apply side)
apply dynasty_span to dynasties {
    lane dynasty;
}
```

| Element | Description |
|---|---|
| `template <name> [as <id>] to <target_type> { ... }` | Defines mapping rules (uses the same properties as `map`) |
| `apply <template_id> to <import_id> { ... }` | Applies a defined template to the specified import |
| `lane <id>;` (inside `apply`) | Overrides the template's `lane` on the apply side |

The properties available in a template are the same as in a `map` block (`lane`, `start`, `end`, `time`, `label`, `tags`, `filter`). Inside `apply`, only `lane` can be overridden.

Complete sample: [`examples/template_apply_example.tdsl`](../examples/template_apply_example.tdsl)

### Expressions

#### claim expression

Retrieves a property value from Wikidata.

```
claim(P571).year    // Convert the time value of P571 (inception) to a year integer
claim(P569).year    // Convert the time value of P569 (date of birth) to a year integer
```

The `??` operator supports claim chains and literal fallbacks (short-circuit evaluation: the right-hand side is only evaluated when the left-hand side cannot be resolved).

```
// claim fallback: use P571 if P580 is missing
start claim(P580).year ?? claim(P571).year;

// literal fallback: use 9999 if P570 is missing
end claim(P570).year ?? 9999;

// chain + literal: try P580, then P571, then 0
time claim(P580).year ?? claim(P571).year ?? 0;
```

#### Qualifier access

Accesses the qualifier properties of a statement.

```
claim(P39).qualifier(P580).year   // Year of qualifier P580 (start time) on the P39 statement
claim(P39).qualifier(P582).year   // Year of qualifier P582 (end time) on the P39 statement
```

If the qualifier does not exist, the expression yields no value (no silent fallback).

#### expand — generate multiple items from multiple statements

Adding `expand claim(P)` inside a `map` block loops over all non-deprecated statements of property P on the entity and generates one item per statement. Without `expand`, only the first statement is consulted, as before.

```tdsl
import wikidata as w {
    entity Q9682 as elizabeth_ii;  // example: Elizabeth II
}

// Example: expand all positions held (P39) into spans
map w.elizabeth_ii to span {
    lane offices;
    expand claim(P39);
    start claim(P39).qualifier(P580).year;
    end   claim(P39).qualifier(P582).year ?? 9999;
    label label@en;
}
```

If there are multiple P39 statements, multiple spans are generated.
Statements missing the qualifiers (P580/P582) skip that item (because `start`/`end` cannot be resolved).

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

> **How comments are handled**: Comments are skipped during parsing and are not retained in the AST or IR. Therefore they are removed when you reformat with `tdsl fmt`, and cannot be restored by `tdsl decompile` (which starts from the IR). This is a design constraint of treating the IR as the single source of truth (comment preservation is tracked in #362).

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

| Format | Example | Precision |
|---|---|---|
| `YYYY` | `1969`, `-206` | Year |
| `YYYY-MM` | `1969-07` | Month |
| `YYYY-MM-DD` | `1969-07-20` | Day |
| `YYYY-MM-DDTHH:MM` | `1969-07-20T20:17` | Minute |

- Range: `start..end` (e.g., `-0206-01-15..-0206-02-20`, `1939-09-01..1945-09-02`, `1969-07-20T20:17..1969-07-20T21:00`)
- BCE dates with month/day or time-of-day precision use a signed 4-digit year (e.g., `-0206-01-15`)
- Wikidata time values can be accessed at any precision using `.year`, `.month`, `.day`, `.hour`, or `.minute`

## CLI

### Subcommand Reference

| Command | Purpose |
|---|---|
| `tdsl build <file>` | Convert `.tdsl` to JSON IR |
| `tdsl check <file>` | Syntax and semantic check |
| `tdsl ast <file>` | Dump the AST |
| `tdsl render <file>` | Generate HTML / SVG / PDF / PNG (`--format html\|svg\|pdf\|png`, `--interactive`) |
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

`tdsl render` outputs the timeline as HTML / SVG / PDF / PNG.

```bash
tdsl render input.tdsl --output timeline.html [--format html|svg|pdf|png] [--interactive] [--scale N] [--offline]
```

| Option | Description |
|---|---|
| `--output` | Output path; defaults to stdout if omitted |
| `--format` | Output format: `html` (default) / `svg` / `pdf` / `png` |
| `--interactive` | Interactive mode with zoom, pan, search, legend, and detail panel (uses JavaScript). Only valid with `--format html` |
| `--show-legend` | Show a static legend panel of lane colors and tag colors (`color_map`) |
| `--scale` | Pixel width per year (default: 2) |
| `--lane-height` | Height of each lane in px (default: 60). Controls vertical density; bar thickness follows it |
| `--layout-style` | High-level visual layout: `timeline` (default) / `group-bands` (background bands for contiguous lane groups) |
| `--dpi` | DPI for PNG output (default: 96). Only valid with `--format png` |
| `--offline` | Skip Wikidata fetch |

### Output specification

- **Formats**:
  - `html`: Single HTML file with inline SVG + CSS. JavaScript-free by default (`--interactive` enables JS)
  - `svg`: Standalone SVG file
  - `pdf`: PDF file (via `svg2pdf` / `usvg`, CJK fonts supported)
  - `png`: PNG raster image (resolution adjustable with `--dpi`)
- **Interactive mode** (`--interactive`): Adds zoom, pan, full-text search, legend, and detail panel. Colors defined in `color_map` are automatically applied
- **Static legend** (`--show-legend`, #544): Renders a legend panel listing each lane's palette color and any `color_map` tag color overrides, independent of `--interactive`. Defaults to `false` (no legend), leaving existing output unchanged.
- **Layout**:
  - Horizontal axis: time (uses `timeline.range`)
  - Vertical axis: lanes stacked top-to-bottom in ascending `order`
  - Axis tick intervals are chosen automatically based on the range (10 / 20 / 50 / 100 years, etc.)
  - `--layout-style group-bands` draws background bands for contiguous lanes sharing the same `lane.group` (#543). This is a render-only option orthogonal to `Orientation`; it does not add IR/DSL fields.
- **Element rendering**:
  - `span` → rounded rectangle (centered in the lane band)
  - `event_range` → narrower rectangle (lower part of the lane band)
  - `event` → vertical line + small circle marker
- **Colors**: Tag colors defined in `timeline { color_map { tag_name: "#hex"; } }` are applied
- **Tooltips**: Hovering over each element shows a `<title>` element with the label, duration, tags, source, and ID

## License and Data Usage

- **Wikidata structured data**: CC0 license. Free to use without attribution.
- **Wikipedia text and figures**: CC BY-SA 4.0. Attribution and same-license application required when quoting.
