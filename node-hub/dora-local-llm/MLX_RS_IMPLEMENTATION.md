# mlx-rs vs mlx-lm: Feature Gap Analysis and Implementation Plan

## Executive Summary

This document analyzes the feature gap between **mlx-lm** (Python) and **mlx-rs** (Rust) for implementing local LLM inference, specifically for Qwen3-8B-4bit on Apple Silicon.

**Key Finding**: The gap is smaller than expected. mlx-rs already contains partial Qwen3 support in `mlx-lm/src/models/qwen3.rs`. We need to create a working example and integrate it.

---

## 1. Architecture Comparison

### mlx-lm (Python) - What It Provides

```
mlx-lm/
├── mlx_lm/
│   ├── models/
│   │   ├── qwen3.py          # Model architecture
│   │   ├── llama.py          # Reference implementation
│   │   └── base.py           # BaseModel class
│   ├── generate.py           # Text generation loop
│   ├── cache.py              # KV cache management
│   ├── sample_utils.py       # Temperature/top-p sampling
│   ├── tokenizer_utils.py    # Tokenizer wrapper
│   └── utils.py              # Model loading utilities
└── examples/
    └── generate.py           # CLI usage example
```

**mlx-lm usage (Python)**:
```python
from mlx_lm import load, generate

model, tokenizer = load("mlx-community/Qwen3-8B-4bit")
response = generate(model, tokenizer, prompt="Hello!", max_tokens=100)
```

### mlx-rs (Rust) - Current State

```
mlx-rs/
├── mlx-rs/                   # Core MLX bindings
│   └── src/
│       ├── ops/              # Tensor operations
│       ├── nn/               # Neural network modules
│       │   ├── linear.rs     # Linear layers
│       │   ├── embedding.rs  # Embeddings
│       │   ├── rope.rs       # Rotary Position Encoding
│       │   └── rms_norm.rs   # RMS Normalization
│       └── module/           # Model abstraction
├── mlx-lm/                   # LLM-specific crate
│   └── src/
│       ├── models/
│       │   ├── qwen3.rs      # ⚠️ PARTIAL - Model architecture
│       │   ├── mistral.rs    # ✅ Complete with example
│       │   └── mod.rs        # Model registry
│       ├── cache.rs          # ✅ KV cache
│       ├── sampler.rs        # ✅ Temperature sampling
│       └── generate.rs       # ✅ Generation loop
└── examples/
    └── mistral/              # ✅ Working Mistral example
        └── src/main.rs
```

---

## 2. Component-by-Component Gap Analysis

| Component | mlx-lm (Python) | mlx-rs (Rust) | Gap Status |
|-----------|-----------------|---------------|------------|
| **Core Tensor Ops** | MLX Python API | mlx-rs bindings | ✅ Complete |
| **Linear Layer** | `nn.Linear` | `mlx_nn::Linear` | ✅ Complete |
| **Embedding** | `nn.Embedding` | `mlx_nn::Embedding` | ✅ Complete |
| **RoPE** | `nn.RoPE` | `mlx_nn::RotaryEmbedding` | ✅ Complete |
| **RMS Norm** | `nn.RMSNorm` | `mlx_nn::RmsNorm` | ✅ Complete |
| **Attention** | `Attention` class | `mlx_lm::Attention` | ✅ Complete |
| **MLP** | `MLP` class | `mlx_lm::Mlp` | ✅ Complete |
| **KV Cache** | `cache.py` | `mlx_lm::cache` | ✅ Complete |
| **Sampling** | `sample_utils.py` | `mlx_lm::sampler` | ✅ Complete |
| **Generation Loop** | `generate.py` | `mlx_lm::generate` | ✅ Complete |
| **Safetensors Loading** | `mx.load()` | `safetensors` crate | ✅ Complete |
| **Tokenizer** | `transformers` | `tokenizers` crate | ✅ Complete |
| **Mistral Model** | Full support | Full support + example | ✅ Complete |
| **Qwen3 Model** | Full support | Architecture only | ⚠️ **Partial** |
| **Qwen3 Example** | CLI tool | None | ❌ **Missing** |
| **HF Hub Download** | `huggingface_hub` | `hf-hub` crate | ✅ Complete |

### Key Gaps Identified

1. **No working Qwen3 example** - The model definition exists but isn't wired up
2. **Weight name mapping** - HuggingFace checkpoint names differ from mlx-rs expectations
3. **Chat template** - Qwen3-specific ChatML format not implemented

---

## 3. Qwen3 Architecture Details

### 3.1 Model Configuration (config.json)

```json
{
  "architectures": ["Qwen3ForCausalLM"],
  "attention_bias": false,
  "head_dim": 128,
  "hidden_size": 4096,
  "intermediate_size": 12288,
  "max_position_embeddings": 40960,
  "num_attention_heads": 32,
  "num_hidden_layers": 36,
  "num_key_value_heads": 8,
  "rms_norm_eps": 1e-06,
  "rope_theta": 1000000.0,
  "vocab_size": 151936
}
```

### 3.2 Qwen3 vs Mistral Architecture Comparison

| Feature | Qwen3 | Mistral | Implementation Impact |
|---------|-------|---------|----------------------|
| **Attention Type** | GQA + QK-Norm | GQA only | Need Q/K normalization layers |
| **QKV Bias** | `false` (no bias) | Has bias | Simpler (fewer params) |
| **RoPE Base (θ)** | 1,000,000 | 10,000 | Config parameter change |
| **Vocab Size** | 151,936 | 32,768 | Larger embedding matrix |
| **Head Dimension** | 128 | 128 | Same |
| **Sliding Window** | None | 4096 | Simpler (no masking) |
| **MLP Activation** | SiLU | SiLU | Same |
| **Norm** | RMSNorm | RMSNorm | Same |

**Conclusion**: Qwen3 is actually **simpler** than Mistral in some aspects (no sliding window, no QKV bias).

### 3.3 Weight Name Mapping (HuggingFace → mlx-rs)

The key challenge is mapping HuggingFace checkpoint weight names to mlx-rs expected names:

```
HuggingFace Checkpoint              →  mlx-rs Expected Name
──────────────────────────────────────────────────────────────
model.embed_tokens.weight           →  token_embeddings.weight
model.layers.{i}.self_attn.q_proj.weight  →  layers.{i}.attention.q_proj.weight
model.layers.{i}.self_attn.k_proj.weight  →  layers.{i}.attention.k_proj.weight
model.layers.{i}.self_attn.v_proj.weight  →  layers.{i}.attention.v_proj.weight
model.layers.{i}.self_attn.o_proj.weight  →  layers.{i}.attention.o_proj.weight
model.layers.{i}.self_attn.q_norm.weight  →  layers.{i}.attention.q_norm.weight  (Qwen3 only)
model.layers.{i}.self_attn.k_norm.weight  →  layers.{i}.attention.k_norm.weight  (Qwen3 only)
model.layers.{i}.mlp.gate_proj.weight     →  layers.{i}.mlp.gate_proj.weight
model.layers.{i}.mlp.up_proj.weight       →  layers.{i}.mlp.up_proj.weight
model.layers.{i}.mlp.down_proj.weight     →  layers.{i}.mlp.down_proj.weight
model.layers.{i}.input_layernorm.weight   →  layers.{i}.input_norm.weight
model.layers.{i}.post_attention_layernorm.weight  →  layers.{i}.post_attention_norm.weight
model.norm.weight                   →  final_norm.weight
lm_head.weight                      →  output_projection.weight
```

### 3.4 Chat Template (ChatML Format)

Qwen3 uses the ChatML format:

```
<|im_start|>system
You are a helpful assistant.<|im_end|>
<|im_start|>user
Hello!<|im_end|>
<|im_start|>assistant
```

**Special Token IDs** (from tokenizer_config.json):
- `<|im_start|>` = 151644
- `<|im_end|>` = 151645
- `<|endoftext|>` = 151643
- `</think>` = 151668 (for thinking/reasoning mode)

---

## 4. Existing mlx-rs Qwen3 Code Analysis

### 4.1 Location

The Qwen3 model is partially implemented at:
```
mlx-rs/mlx-lm/src/models/qwen3.rs  (19.3 KB)
```

### 4.2 Current Implementation Status

```rust
// mlx-lm/src/models/qwen3.rs

/// Qwen3 Attention with QK-Norm support
pub struct Attention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    o_proj: Linear,
    q_norm: RmsNorm,  // ✅ QK-Norm included
    k_norm: RmsNorm,  // ✅ QK-Norm included
    rope: RotaryEmbedding,
    // ...
}

/// Qwen3 MLP (SwiGLU)
pub struct Mlp {
    gate_proj: Linear,
    up_proj: Linear,
    down_proj: Linear,
}

/// Qwen3 Decoder Layer
pub struct DecoderLayer {
    self_attn: Attention,
    mlp: Mlp,
    input_layernorm: RmsNorm,
    post_attention_layernorm: RmsNorm,
}

/// Qwen3 Model
pub struct Qwen3Model {
    embed_tokens: Embedding,
    layers: Vec<DecoderLayer>,
    norm: RmsNorm,
    lm_head: Linear,
}

impl Model for Qwen3Model {
    fn forward(&self, input: &ModelInput) -> Array {
        // Model forward pass implementation
    }
}
```

### 4.3 What's Missing from qwen3.rs

1. **`load()` function** - No weight loading from safetensors
2. **Config parsing** - Need to read `config.json` and create model
3. **Weight sanitization** - HF → mlx-rs name mapping
4. **Working example** - No main.rs showing usage

---

## 5. Implementation Plan

### Phase 1: Create Working Example (1-2 days)

Create `mlx-rs/examples/qwen3/` following the Mistral pattern:

```
examples/qwen3/
├── Cargo.toml
├── src/
│   ├── main.rs          # CLI entry point
│   ├── model.rs         # Qwen3 model wrapper (uses mlx_lm::models::qwen3)
│   └── weights.rs       # Weight name mapping
└── README.md
```

**main.rs structure**:
```rust
use clap::Parser;
use hf_hub::api::sync::Api;
use mlx_lm::models::qwen3::Qwen3Model;
use tokenizers::Tokenizer;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "mlx-community/Qwen3-8B-4bit")]
    model: String,

    #[arg(long)]
    prompt: String,

    #[arg(long, default_value = "100")]
    max_tokens: usize,

    #[arg(long, default_value = "0.7")]
    temperature: f32,
}

fn main() -> eyre::Result<()> {
    let args = Args::parse();

    // 1. Download model from HuggingFace
    let api = Api::new()?;
    let repo = api.model(args.model);
    let model_path = repo.get("model.safetensors")?;
    let tokenizer_path = repo.get("tokenizer.json")?;
    let config_path = repo.get("config.json")?;

    // 2. Load config
    let config: Qwen3Config = serde_json::from_str(&std::fs::read_to_string(config_path)?)?;

    // 3. Load tokenizer
    let tokenizer = Tokenizer::from_file(tokenizer_path)?;

    // 4. Load model weights with name mapping
    let weights = load_safetensors_with_mapping(&model_path)?;
    let model = Qwen3Model::from_weights(weights, &config)?;

    // 5. Apply chat template
    let prompt = apply_chat_template(&args.prompt);

    // 6. Tokenize
    let input_ids = tokenizer.encode(prompt, false)?.get_ids().to_vec();

    // 7. Generate
    let mut cache = KvCache::new(&config);
    let output_ids = generate(&model, &input_ids, args.max_tokens, args.temperature, &mut cache)?;

    // 8. Decode and print
    let response = tokenizer.decode(&output_ids, true)?;
    println!("{}", response);

    Ok(())
}
```

### Phase 2: Weight Loading Integration (1 day)

Create weight name mapping function:

```rust
// weights.rs

use std::collections::HashMap;
use mlx_rs::Array;

/// Map HuggingFace Qwen3 checkpoint names to mlx-rs expected names
pub fn sanitize_qwen3_weights(
    weights: HashMap<String, Array>
) -> HashMap<String, Array> {
    weights.into_iter()
        .map(|(name, tensor)| {
            let new_name = map_weight_name(&name);
            (new_name, tensor)
        })
        .collect()
}

fn map_weight_name(hf_name: &str) -> String {
    // Handle embedding
    if hf_name == "model.embed_tokens.weight" {
        return "token_embeddings.weight".to_string();
    }

    // Handle lm_head
    if hf_name == "lm_head.weight" {
        return "output_projection.weight".to_string();
    }

    // Handle final norm
    if hf_name == "model.norm.weight" {
        return "final_norm.weight".to_string();
    }

    // Handle layer weights: model.layers.{i}.* → layers.{i}.*
    if let Some(rest) = hf_name.strip_prefix("model.layers.") {
        let parts: Vec<&str> = rest.splitn(2, '.').collect();
        if parts.len() == 2 {
            let layer_idx = parts[0];
            let component = parts[1];

            // Map self_attn.* → attention.*
            let new_component = component
                .replace("self_attn.", "attention.")
                .replace("input_layernorm", "input_norm")
                .replace("post_attention_layernorm", "post_attention_norm");

            return format!("layers.{}.{}", layer_idx, new_component);
        }
    }

    // Default: return unchanged
    hf_name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weight_mapping() {
        assert_eq!(
            map_weight_name("model.embed_tokens.weight"),
            "token_embeddings.weight"
        );
        assert_eq!(
            map_weight_name("model.layers.0.self_attn.q_proj.weight"),
            "layers.0.attention.q_proj.weight"
        );
        assert_eq!(
            map_weight_name("model.layers.15.mlp.gate_proj.weight"),
            "layers.15.mlp.gate_proj.weight"
        );
    }
}
```

### Phase 3: Chat Template (0.5 day)

```rust
// chat_template.rs

/// Apply Qwen3 ChatML template to messages
pub fn apply_chat_template(messages: &[Message]) -> String {
    let mut prompt = String::new();

    for msg in messages {
        prompt.push_str("<|im_start|>");
        prompt.push_str(&msg.role);
        prompt.push('\n');
        prompt.push_str(&msg.content);
        prompt.push_str("<|im_end|>\n");
    }

    // Add assistant prompt start
    prompt.push_str("<|im_start|>assistant\n");

    prompt
}

pub struct Message {
    pub role: String,    // "system", "user", "assistant"
    pub content: String,
}

/// Get stop tokens for Qwen3
pub fn get_stop_tokens() -> Vec<u32> {
    vec![
        151645,  // <|im_end|>
        151643,  // <|endoftext|>
    ]
}
```

### Phase 4: Integrate into dora-local-llm (1 day)

Update `node-hub/dora-local-llm/src/inference.rs`:

```rust
use mlx_lm::models::qwen3::Qwen3Model;
use mlx_lm::cache::KvCache;
use mlx_lm::generate::generate;
use tokenizers::Tokenizer;

pub struct Inference {
    model: Qwen3Model,
    tokenizer: Tokenizer,
    config: Qwen3Config,
}

impl Inference {
    pub fn new(model_path: &str) -> eyre::Result<Self> {
        // Load config
        let config_path = format!("{}/config.json", model_path);
        let config: Qwen3Config = serde_json::from_str(
            &std::fs::read_to_string(&config_path)?
        )?;

        // Load tokenizer
        let tokenizer_path = format!("{}/tokenizer.json", model_path);
        let tokenizer = Tokenizer::from_file(&tokenizer_path)?;

        // Load and sanitize weights
        let weights = load_safetensors(&format!("{}/model.safetensors", model_path))?;
        let weights = sanitize_qwen3_weights(weights);

        // Create model
        let model = Qwen3Model::from_weights(weights, &config)?;

        Ok(Self { model, tokenizer, config })
    }

    pub fn generate(
        &self,
        messages: &[Message],
        max_tokens: usize,
        temperature: f32,
        cache: &mut Option<KvCache>,
    ) -> eyre::Result<String> {
        // Apply chat template
        let prompt = apply_chat_template(messages);

        // Tokenize
        let encoding = self.tokenizer.encode(prompt, false)?;
        let input_ids = encoding.get_ids().to_vec();

        // Get or create cache
        let kv_cache = cache.get_or_insert_with(|| KvCache::new(&self.config));

        // Generate
        let output_ids = mlx_lm::generate::generate(
            &self.model,
            &input_ids,
            max_tokens,
            temperature,
            kv_cache,
            &get_stop_tokens(),
        )?;

        // Decode
        let response = self.tokenizer.decode(&output_ids, true)?;

        Ok(response)
    }
}
```

---

## 6. Testing Strategy

### 6.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weight_name_mapping() {
        // Test all weight name transformations
    }

    #[test]
    fn test_chat_template() {
        let messages = vec![
            Message { role: "system".into(), content: "You are helpful.".into() },
            Message { role: "user".into(), content: "Hello!".into() },
        ];
        let result = apply_chat_template(&messages);
        assert!(result.contains("<|im_start|>system"));
        assert!(result.ends_with("<|im_start|>assistant\n"));
    }

    #[test]
    fn test_stop_tokens() {
        let tokens = get_stop_tokens();
        assert!(tokens.contains(&151645)); // <|im_end|>
    }
}
```

### 6.2 Integration Test

```bash
# Download model (one-time)
huggingface-cli download mlx-community/Qwen3-8B-4bit

# Run example
cargo run --example qwen3 -- \
    --model ~/.cache/huggingface/hub/models--mlx-community--Qwen3-8B-4bit \
    --prompt "Hello, how are you?" \
    --max-tokens 100 \
    --temperature 0.7
```

### 6.3 Performance Benchmark

Compare with Python mlx-lm:

```python
# benchmark_python.py
import time
from mlx_lm import load, generate

model, tokenizer = load("mlx-community/Qwen3-8B-4bit")

start = time.time()
for _ in range(10):
    generate(model, tokenizer, prompt="Hello!", max_tokens=50)
print(f"Python mlx-lm: {(time.time() - start) / 10:.3f}s per generation")
```

```bash
# benchmark_rust.sh
hyperfine --warmup 1 --runs 10 \
    'cargo run --release --example qwen3 -- --prompt "Hello!" --max-tokens 50'
```

---

## 7. Timeline Summary

| Phase | Task | Duration | Deliverable |
|-------|------|----------|-------------|
| 1 | Create qwen3 example | 1-2 days | Working CLI tool |
| 2 | Weight loading | 1 day | Name mapping function |
| 3 | Chat template | 0.5 day | ChatML implementation |
| 4 | dora-local-llm integration | 1 day | Real inference in server |
| **Total** | | **3.5-4.5 days** | **Pure Rust Qwen3** |

---

## 8. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| mlx-rs qwen3.rs incomplete | Medium | High | Fall back to adapting Mistral code |
| Weight mapping incorrect | Medium | Medium | Use Python mlx-lm as reference |
| Performance regression | Low | Medium | Profile and optimize hot paths |
| Memory issues | Low | High | Test with 4-bit quantization first |

---

## 9. References

### mlx-rs Repository
- Main repo: https://github.com/oxideai/mlx-rs
- Qwen3 model: `mlx-lm/src/models/qwen3.rs`
- Mistral example: `examples/mistral/`

### mlx-lm Python Reference
- Main repo: https://github.com/ml-explore/mlx-examples
- Qwen3 model: `mlx_lm/models/qwen3.py`
- Generate: `mlx_lm/generate.py`

### Qwen3 Model Card
- HuggingFace: https://huggingface.co/mlx-community/Qwen3-8B-4bit
- Config: `config.json`, `tokenizer_config.json`

---

## 10. Conclusion

The mlx-rs ecosystem is **closer to full Qwen3 support than initially thought**. The model architecture already exists in `mlx-lm/src/models/qwen3.rs`. The main work is:

1. Creating a working example that wires everything together
2. Implementing weight name mapping for HuggingFace checkpoints
3. Adding Qwen3-specific chat template

With 3.5-4.5 days of focused work, we can have **pure Rust Qwen3 inference** integrated into dora-local-llm, eliminating the need for the Python server entirely.
