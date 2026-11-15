# Read-only presence checks; these do not verify source SHA or publish anything.
# hurl --test --continue-on-error --variable version=2.1.0 docs/release-checks.hurl
GET https://registry.npmjs.org/@keroway%2Ftdsl-wasm/{{version}}
HTTP 200
[Asserts]
jsonpath "$.version" == "{{version}}"

GET https://crates.io/api/v1/crates/tdsl-parser/{{version}}
User-Agent: timeline-dsl-readiness-check
HTTP 200
[Asserts]
jsonpath "$.version.num" == "{{version}}"

GET https://crates.io/api/v1/crates/tdsl-wikidata/{{version}}
User-Agent: timeline-dsl-readiness-check
HTTP 200
[Asserts]
jsonpath "$.version.num" == "{{version}}"

GET https://crates.io/api/v1/crates/tdsl-core/{{version}}
User-Agent: timeline-dsl-readiness-check
HTTP 200
[Asserts]
jsonpath "$.version.num" == "{{version}}"

GET https://crates.io/api/v1/crates/tdsl-render/{{version}}
User-Agent: timeline-dsl-readiness-check
HTTP 200
[Asserts]
jsonpath "$.version.num" == "{{version}}"

GET https://marketplace.visualstudio.com/_apis/public/gallery/publishers/keroway/vsextensions/timeline-dsl/{{version}}/vspackage
HTTP 200
