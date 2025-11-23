# Dora Node Hub

This hub contains useful pre-built nodes for Dora.




# Python

## Add a new python node

- To work on a new node, start by:

```bash
cd node-hub
dora new your-node-name --lang python --kind node
cd ./your-node-name
uv venv --seed -p 3.11
uv pip install -e . # Install
uv run ruff check . --fix # Format
uv run ruff check . # Lint
uv run pytest . # Test
```

- To add a python dependency just do:

```bash
uv add numpy # for example
```

> The package is then added to your `pyproject.toml`

- Modify the code within `main.py` in your liking.

- Create a PR and let the CI/CD run test on it 🙋

## Structure

The structure of the node hub is as follows (please use the same structure if you need to add a new node):

```
node-hub/
└── your-node/
    ├── README.md
    ├── your-node
    │   ├── __init__.py
    │   ├── __main__.py
    │   └── main.py
    ├── pyproject.toml
    └── tests
        └── test_<your-node>.py
```

The idea is to make a `pyproject.toml` file that will install the required dependencies for the node **and** attach main
function of the node inside a callable script in your environment.

To do so, you will need to add a `main` function inside the `main.py` file.

```python
def main():
    pass
```

And then you will need to adapt the following `pyproject.toml` file:

```toml
[project]
name = "[name of the node e.g. video-encoder, with '-' to replace spaces]"
version = "0.1"
authors = [{ name = "[Pseudo/Name]", email = "[email]" }]
description = "Dora Node for []"
readme = "README.md"
license = { text = "MIT" }

dependencies = [
    "dora-rs >= 0.3.8",
]

[project.scripts]
[name of the node with '-' to replace spaces] = "[name of the node with '_' to replace spaces].main:main"

[tool.ruff.lint]
extend-select = [
  "D",    # pydocstyle
  "UP",   # Ruff's UP rule
  "PERF", # Ruff's PERF rule
  "RET",  # Ruff's RET rule
  "RSE",  # Ruff's RSE rule
  "NPY",  # Ruff's NPY rule
  "N",    # Ruff's N rule
  "I",    # Ruff's I rule
]
```

Finally, the README.md file should explicit all inputs/outputs of the node and how to configure it in the YAML file.

## Example

```toml
[project]
name = "opencv-plot"
version = "0.1"
authors = [
    "Haixuan Xavier Tao <tao.xavier@outlook.com>",
    "Enzo Le Van <dev@enzo-le-van.fr>"
]
description = "Dora Node for plotting data with OpenCV"
readme = "README.md"
license = { text = "MIT" }
requires-python = ">=3.7"

dependencies = [
    "dora-rs >= 0.3.8",
]
[dependency-groups]
dev = ["pytest >=8.1.1", "ruff >=0.9.1"]

[project.scripts]
opencv-plot = "opencv_plot.main:main"

[tool.ruff.lint]
extend-select = [
  "D",    # pydocstyle
  "UP",   # Ruff's UP rule
  "PERF", # Ruff's PERF rule
  "RET",  # Ruff's RET rule
  "RSE",  # Ruff's RSE rule
  "NPY",  # Ruff's NPY rule
  "N",    # Ruff's N rule
  "I",    # Ruff's I rule
]
```
## Adding git dependency
- If a git repository is added as submodule. Proper path should be added in `pyproject.toml` inorder to make sure that linting and testing are exempted for that dependency.
- A very good example of how this can be done is as follows

Correct approach:
```toml
[tool.ruff]
exclude = ["dora_magma/Magma"]

[tool.black]
extend.exclude = "dora_magma/Magma"
```
Incorrect Approach:
```toml
[tool.ruff]
exclude = ["dora-magma/dora_magma/Magma"]

[tool.black]
extend.exclude = "dora_magma/Magma"
```
##### Note:
- `dora-magma` is root folder of the node.

# Rust

## Add a new rust node

```bash
cd node-hub
dora new your-node-name --lang rust --kind node
cd ./your-node-name
```

## Steps Before Building

- Before building the node, make sure to add your node to the workspace members list in the root `Cargo.toml` file:

```
[workspace]
members = [
...
"node-hub/your-node-name"
]
```

- Also change the `Cargo.toml` file in your node to use the workspace version of dora-node-api:

```
[dependencies]
dora-node-api = { workspace = true }
```

## Structure

The structure of the node hub for Rust is as follows (please use the same structure if you need to add a new node):

```
node-hub/
└── your-node/
    ├── Cargo.toml
    ├── README.md
    └── src/
           └── main.rs
```

The README.md file should explicit all inputs/outputs of the node and how to configure it in the YAML file.

## 📚 Essential Node Documentation

### Rust Nodes with API Specifications

#### [dora-maas-client](./dora-maas-client/)
Cloud AI integration node with multi-provider support (OpenAI, Gemini, etc.)
- **Features**: Streaming, session management, MCP tools, request cancellation
- **📖 API Spec**: [dora-maas-client/API.md](./dora-maas-client/API.md)
- **Use Cases**: Chatbots, voice assistants, multi-agent systems

#### [dora-conference-bridge](./dora-conference-bridge/)
Multi-participant coordination node for conference/debate scenarios
- **Features**: Bundled/cold-start modes, dynamic ports, session_status-based completion, cancellation handling
- **📖 API Spec**: [dora-conference-bridge/API.md](./dora-conference-bridge/API.md)
- **Use Cases**: Multi-person conferences, debates, multi-agent coordination

### Key Python Nodes

#### [dora-asr](./dora-asr/)
Speech recognition with GPU acceleration and multi-engine support (FunASR, Whisper)
- **Features**: GPU acceleration (2.27x faster), Chinese/English support, intelligent engine selection
- **Use Cases**: Voice chatbots, transcription, real-time ASR

#### [dora-primespeech](./dora-primespeech/)
High-quality text-to-speech with G2PW phonetic model and enhanced error handling
- **Features**: Chinese TTS, complete G2PW model, error recovery with tracebacks
- **Use Cases**: Voice synthesis, pronunciation correction

#### [dora-openai-websocket](./dora-openai-websocket/)
WebSocket server for OpenAI-compatible API endpoints
- **Features**: WebSocket streaming, real-time audio/text exchange
- **Use Cases**: Web-based voice chat, browser integration

#### [dora-mcp-host](./dora-mcp-host/) and [dora-mcp-server](./dora-mcp-server/)
Model Context Protocol (MCP) integration for tool use
- **Features**: Tool registration, execution, pass-through to LLMs
- **Use Cases**: Function calling, API integration, knowledge retrieval

## 📖 Node API Documentation Guidelines

When creating a new node, please include a comprehensive `API.md` file that documents:

- **Input ports**: Name, type, metadata fields, required/optional
- **Output ports**: Name, type, metadata fields, data format
- **Configuration**: Environment variables, config files, defaults
- **Examples**: Dataflow configurations, usage patterns
- **Special behaviors**: Completion detection, error handling, edge cases

See [dora-maas-client/API.md](./dora-maas-client/API.md) for a comprehensive example.

## License

This project is licensed under Apache-2.0. Check out [NOTICE.md](../NOTICE.md) for more information.
