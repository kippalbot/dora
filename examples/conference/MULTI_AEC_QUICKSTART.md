# Multi-Party Study System with Human Speaker Quick Start Guide
# 多方学习系统（含人类说话者）快速入门指南

## Prerequisites / 前置条件

### System Requirements / 系统要求
- Python 3.12+
- Rust (for building Dora components)
- 16GB RAM recommended
- 8-core CPU (4-core minimum)
- macOS / Linux (Windows WSL2)

### API Keys / API密钥
```bash
export OPENAI_API_KEY="your-key-here"
export DEEPSEEK_API_KEY="your-key-here"  
export ALIBABA_CLOUD_API_KEY="your-key-here"
```

## Quick Start / 快速开始

### 1. Build Dora Components / 构建Dora组件
```bash
cd /Users/yuechen/home/fresh/dora
cargo build --release -p dora-conference-controller
cargo build --release -p dora-conference-bridge
cargo build --release -p dora-maas-client
```

### 2. Install Python Dependencies / 安装Python依赖
```bash
pip install -e node-hub/dora-text-segmenter
pip install -e node-hub/dora-primespeech
```

### 3. Download Models / 下载模型
```bash
# PrimeSpeech TTS models
python examples/model-manager/download_models.py --model primespeech
```

### 4. Start the Dataflow / 启动数据流

#### Option A: Full System with Audio (Recommended)
```bash
cd examples/conference
dora up
dora start dataflow-study-audio-multi.yml --name study-session
```

#### Option B: Text-Only (No Audio)
```bash
cd examples/conference
dora up
dora start dataflow-study-sequential.yml --name study-session
```

### 5. Launch Dynamic Components / 启动动态组件

The dataflow requires several dynamic Python nodes to be started manually:

#### Terminal 1: Study Monitor (TUI)
```bash
cd examples/conference
python debate_monitor.py
```
**Purpose**: Interactive terminal UI for monitoring and controlling the study session
- View real-time participant responses
- See audio buffer status
- Control session flow (pause/resume/reset)

#### Terminal 2: Viewer (Logging)
```bash
cd examples/conference
python debate_viewer.py
```
**Purpose**: Comprehensive logging display
- All component logs in one view
- Status updates from bridges, controller, LLMs
- Debug information for troubleshooting

#### Terminal 3: Audio Player (For Audio Mode)
```bash
cd examples/conference
python audio_player.py --buffer-seconds 30
```
**Purpose**: Multi-participant audio playback
- Plays synthesized speech from all 3 participants
- Manages audio buffer and backpressure (30 seconds buffer)
- Sends completion signals to segmenter

**Parameters**:
- `--buffer-seconds 30` - Audio buffer size (default: 30 seconds)
  - Increase (e.g., 60) if experiencing buffer overflow
  - Decrease (e.g., 15) for lower latency with risk of underflow

#### Terminal 4: MAC-AEC Segmentation (For Human Input)
```bash
cd examples/mac-aec-chat
python mac_aec_simple_segmentation.py
```
**Purpose**: Human speaker input processing
- Captures microphone input with echo cancellation
- Performs speech recognition (ASR)
- Sends transcribed text to controller

### 6. Start the Study Session / 开始学习会话

Once all dynamic components are running, initiate the study session:

#### Using Study Monitor (debate_monitor.py):

1. **The monitor displays a text input area at the bottom**

2. **Type your initial prompt/question**, for example:
   ```
   请解释量子纠缠的基本原理和应用
   (Please explain the basic principles and applications of quantum entanglement)
   ```

3. **Press Enter or click "Send" to submit**

4. **The system will:**
   - Send your prompt to the Tutor (first speaker)
   - Tutor generates response
   - Student1 responds to Tutor
   - Student2 responds to both
   - Audio plays through speakers (if audio mode enabled)

5. **Monitor the conversation in the TUI:**
   - Top panels show participant responses in real-time
   - Bottom panel shows audio buffer status
   - Status indicators show which participant is currently speaking

#### Using Human Voice Input (with MAC-AEC):

1. **Ensure mac_aec_simple_segmentation.py is running**

2. **Speak into your microphone**:
   - The system automatically detects speech
   - Applies echo cancellation to remove speaker feedback
   - Transcribes your speech to text
   - Sends to controller as human input

3. **The system will:**
   - Interrupt any ongoing AI conversation
   - Reset to a new round with incremented question_id
   - Start fresh discussion based on your question

### 7. Monitor and Control / 监控和控制

#### Study Monitor Controls:

**Keyboard Shortcuts**:
- `Ctrl+C` - Stop monitoring (graceful exit)
- `Ctrl+R` - Reset current session
- `Space` - Pause/Resume audio playback
- `↑/↓` - Scroll through conversation history

**Visual Indicators**:
```
┌─ Student1 ─────────────────────────┐
│ [ACTIVE] Generating response...    │
│ Text appears here as it streams... │
└────────────────────────────────────┘

┌─ Audio Buffer ─────────────────────┐
│ [45.2%] NORMAL ████████░░░░░░░░    │
└────────────────────────────────────┘
```

#### Viewer Log Categories:

The viewer shows color-coded logs:
- 🔵 **INFO** - Normal operation events
- 🟡 **DEBUG** - Detailed processing information
- 🔴 **ERROR** - Issues requiring attention
- 🟢 **STATUS** - Component state changes

### 8. Interact with the System / 与系统交互

#### Text Input (via Study Monitor):

**Single-Turn Questions**:
```
Q: 什么是机器学习？
(What is machine learning?)
```

**Multi-Part Discussions**:
```
Q: 讨论人工智能的伦理问题，特别关注隐私和偏见。
(Discuss the ethical issues of AI, focusing on privacy and bias.)
```

**Follow-Up Questions**:
```
Q: 基于前面的讨论，如何解决数据偏见问题？
(Based on the previous discussion, how to solve data bias issues?)
```

#### Voice Input (via MAC-AEC):

**Natural Speech**:
- Simply speak naturally into your microphone
- Pause briefly between sentences for better recognition
- The system handles echo cancellation automatically

**Interrupt Capability**:
- Speak anytime during AI responses to interrupt
- System resets and starts new round with your input
- Previous conversation context is preserved

### 9. Stop the System / 停止系统

#### Graceful Shutdown Sequence:

1. **Stop dynamic components** (in reverse order):
   ```bash
   # In each terminal, press Ctrl+C:
   # Terminal 4: mac_aec_simple_segmentation.py
   # Terminal 3: audio_player.py
   # Terminal 2: debate_viewer.py
   # Terminal 1: debate_monitor.py
   ```

2. **Stop the dataflow**:
   ```bash
   dora destroy
   ```

3. **Stop the daemon**:
   ```bash
   dora down
   ```

#### Quick Stop (If Needed):
```bash
# Kill all dora processes
pkill -9 -f dora

# Clean up
dora down
```

## System Components / 系统组件

```
Human → MAC-AEC → ASR → Controller → Bridges → LLMs → Segmenter → TTS → Audio Player → Speakers
人类 → 回声消除 → 语音识别 → 控制器 → 桥接器 → LLM → 分割器 → TTS → 音频播放器 → 扬声器
```

## Configuration Files / 配置文件

| File | Purpose |
|------|---------|
| `dataflow-study-multi-aec.yml` | Main dataflow configuration / 主数据流配置 |
| `study_config_maas_student1.toml` | Student1 LLM configuration / 学生1配置 |
| `study_config_maas_student2.toml` | Student2 LLM configuration / 学生2配置 |
| `study_config_maas_tutor.toml` | Tutor LLM configuration / 导师配置 |

## Common Issues / 常见问题

### Audio Not Playing / 音频不播放
```bash
# Check audio device
python -c "import sounddevice as sd; print(sd.query_devices())"

# Restart audio-player
dora destroy && dora start dataflow-study-multi-aec.yml
```

### LLM Timeout / LLM超时
- Check API keys / 检查API密钥
- Check network connection / 检查网络连接
- Increase timeout in config / 增加配置中的超时时间

### Buffer Overflow / 缓冲区溢出
- Lower `AUDIO_BUFFER_THRESHOLD` from 30 to 20
- Increase `buffer_seconds` from 30 to 60

## Next Steps / 下一步

1. Read full architecture: `MULTI_AEC_ARCHITECTURE.md`
2. Customize personas in `study_config_maas_*.toml`
3. Adjust speaking order in `DORA_POLICY_PATTERN`
4. Tune audio parameters for your hardware

## Support / 支持

- GitHub Issues: https://github.com/kippalbot/dora/issues
- Documentation: See `MULTI_AEC_ARCHITECTURE.md`
