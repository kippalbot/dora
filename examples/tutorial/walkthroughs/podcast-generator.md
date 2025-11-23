# Walkthrough: podcast-generator

Use this guide to generate long-form audio programs by chaining planning, scripting, and speech synthesis nodes.

## 1. Prepare Content Inputs
- Supply topic prompts via `inputs/topics.json`; each entry seeds the planning LLM.
- Optional: add guest profiles or sponsor messages under `inputs/snippets/` to weave into generated episodes.

## 2. Understand the Chained Pipeline
- The dataflow splits into stages: planner LLM → script generator LLM → segment scheduler → TTS cluster.
- Inspect `examples/podcast-generator/dataflow.yml` and the helper nodes in `podcast-generator/nodes/` to see how messages move across stages.

## 3. Execute the Flow
- Build artifacts with `dora build --dataflow dataflow.yml` and verify the DOT graph for multi-stage queues.
- Launch using `dora start --dataflow dataflow.yml`; monitor `output/episodes/` for generated WAV files and metadata.

## 4. Optimize & Iterate
- Swap in local LLM/TTS backends by following `requirements/ominix-local.md`; update configuration files accordingly.
- Introduce interruption or branching logic by inserting queue control nodes between the planner and script generator.
- Record lessons learned about narration pacing, TTS voice blending, and batching throughput at the bottom of this document.
