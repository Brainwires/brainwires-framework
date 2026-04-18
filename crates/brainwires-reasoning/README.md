# brainwires-reasoning

Structured reasoning primitives for the [Brainwires Agent Framework](https://github.com/Brainwires/brainwires-framework).

Re-exports plan parsing and output parsing utilities from `brainwires-core` as a focused, discoverable crate for Layer 3 consumers.

## Features

- **plan_parser** — Extract numbered task steps from LLM plan output
- **output_parser** — Parse structured data (JSON, regex) from raw LLM text

## Usage

```toml
[dependencies]
brainwires-reasoning = "0.10"
```

```rust
use brainwires_reasoning::{parse_plan_steps, JsonOutputParser, OutputParser};

// Parse a numbered plan from LLM output
let steps = parse_plan_steps("1. Research topic\n2. Write summary\n3. Review");

// Parse JSON from LLM output
let parser = JsonOutputParser::new();
let value = parser.parse("```json\n{\"result\": 42}\n```")?;
```

## License

Apache-2.0 — see [LICENSE](../../LICENSE) for details.
