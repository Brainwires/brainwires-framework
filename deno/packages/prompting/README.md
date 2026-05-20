# @brainwires/prompting

Adaptive prompting techniques + task clustering + temperature optimization.

Extracted from `@brainwires/knowledge` in v0.11.0 to mirror Rust's
`brainwires-prompting` crate.

Contents:

- 15 prompting techniques with effectiveness tracking
- Task clustering (k-means over embedding space)
- Dynamic prompt generation per task cluster
- Temperature optimization per technique
- Learning coordinator for SEAL-driven adaptation
