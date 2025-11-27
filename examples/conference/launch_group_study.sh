#!/bin/bash

# 群体学习系统启动指南 | Group Study System Launch Guide
# 此脚本提供详细的启动指导，但需要手动在新终端中执行命令 | This script provides detailed startup guidance but requires manual execution in new terminals

set -e

# 颜色定义 | Color Definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_step() {
    echo -e "${PURPLE}[STEP]${NC} $1"
}

log_command() {
    echo -e "${CYAN}[COMMAND]${NC} $1"
}

clear
echo
log_info "=== 群体学习系统启动指南 | Group Study System Launch Guide ==="
echo

# 显示启动说明 | Show startup instructions
echo "此系统需要4个终端来运行所有组件 | This system requires 4 terminals to run all components"
echo
echo "请按照以下步骤在4个不同的终端中执行命令 | Please follow these steps in 4 different terminals:"
echo

# 检查环境 | Check environment
check_environment() {
    log_step "检查环境配置 | Checking environment configuration..."

    # 检查当前目录 | Check current directory
    if [[ ! -f "dataflow-study-audio-multi.yml" ]]; then
        log_error "请在正确的目录中运行此脚本 | Please run this script in the correct directory"
        echo "目录应包含: dataflow-study-audio-multi.yml | Directory should contain: dataflow-study-audio-multi.yml"
        exit 1
    fi

    # 检查必要文件 | Check required files
    local required_files=("audio_player.py" "debate_monitor.py" "debate_viewer.py" "initial-prompt.md")
    for file in "${required_files[@]}"; do
        if [[ ! -f "$file" ]]; then
            log_error "缺少必要文件 | Missing required file: $file"
            exit 1
        fi
    done

    # 检查环境变量 | Check environment variables
    if [[ -z "$ALIBABA_CLOUD_API_KEY" && -z "$OPENAI_API_KEY" && -z "$DEEPSEEK_API_KEY" ]]; then
        log_warning "未检测到API密钥 | No API keys detected"
        echo "请设置以下环境变量之一 | Please set one of the following:"
        echo "  export ALIBABA_CLOUD_API_KEY='your_key'"
        echo "  export OPENAI_API_KEY='your_key'"
        echo "  export DEEPSEEK_API_KEY='your_key'"
        echo
        read -p "是否继续？| Continue? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi

    log_success "环境检查完成 | Environment check completed"
}

# 显示所有终端命令 | Show all terminal commands
show_all_commands() {
    echo
    log_step "终端启动命令 | Terminal Startup Commands"
    echo
    echo "请在4个不同终端中依次执行以下命令 | Please execute the following commands in 4 different terminals:"
    echo

    echo -e "${CYAN}📍 终端 1 | Terminal 1: 启动数据流服务 | Start Dataflow Service${NC}"
    log_command "cd $(pwd)"
    log_command "dora start dataflow-study-audio-multi.yml"
    echo

    echo -e "${CYAN}📍 终端 2 | Terminal 2: 启动音频播放器 | Start Audio Player${NC}"
    log_command "cd $(pwd)"
    log_command "python audio_player.py --buffer-seconds 300"
    echo

    echo -e "${CYAN}📍 终端 3 | Terminal 3: 启动监控界面 | Start Monitor Interface${NC}"
    log_command "cd $(pwd)"
    log_command "python debate_monitor.py"
    echo

    echo -e "${CYAN}📍 终端 4 | Terminal 4: 启动日志查看器 | Start Log Viewer (可选 | optional)${NC}"
    log_command "cd $(pwd)"
    log_command "python debate_viewer.py"
    echo

    echo -e "${YELLOW}💡 提示 | Tip: 等待每个服务完全启动后再启动下一个 | Wait for each service to fully start before starting the next${NC}"
    echo
}

# 显示冷启动指南 | Show cold-start guide
show_cold_start_guide() {
    echo
    log_step "对话冷启动指南 | Conversation Cold-Start Guide"
    echo
    echo -e "${CYAN}📝 步骤 1 | Step 1: 复制初始提示词 | Copy Initial Prompt${NC}"
    log_command "cat initial-prompt.md"
    echo
    echo "或者直接复制以下内容 | Or copy the following content:"
    echo "----------------------------------------"
    cat initial-prompt.md
    echo "----------------------------------------"
    echo

    echo -e "${CYAN}🎯 步骤 2 | Step 2: 在Study Monitor中发送 | Send in Study Monitor${NC}"
    echo "1. 在debate_monitor界面的底部输入框中粘贴提示词 | Paste in debate_monitor bottom input field"
    echo "2. 按 Tab 键选择 'Send' 按钮 | Press Tab to select 'Send' button"
    echo "3. 按 Enter 发送给tutor开始对话 | Press Enter to send to tutor"
    echo

    echo -e "${CYAN}✅ 步骤 3 | Step 3: 验证启动 | Verify Startup${NC}"
    echo "- 观察tutor面板显示思考状态 | Observe tutor panel showing thinking status"
    echo "- 等待tutor的语音响应 | Wait for tutor's audio response"
    echo "- 检查音频播放是否正常 | Check if audio playback works"
    echo
}

# 显示控制说明 | Show control instructions
show_control_instructions() {
    echo
    log_step "控制说明 | Control Instructions"
    echo
    echo -e "${GREEN}⌨️  Study Monitor控制键 | Study Monitor Control Keys:${NC}"
    echo "  r - 重置当前讨论 | Reset current discussion"
    echo "  c - 取消当前发言 | Cancel current speaking"
    echo "  n - 新问题 | New question"
    echo "  q - 退出程序 | Quit program"
    echo "  Tab - 在Send按钮间切换 | Switch between Send buttons"
    echo "  Enter - 发送消息 | Send message"
    echo

    echo -e "${RED}🛑 停止系统 | Stop System:${NC}"
    echo "1. 在终端1中按 Ctrl+C 停止数据流 | Press Ctrl+C in Terminal 1 to stop dataflow"
    echo "2. 在终端2中按 Ctrl+C 停止音频播放器 | Press Ctrl+C in Terminal 2 to stop audio player"
    echo "3. 在终端3中按 Ctrl+C 停止监控界面 | Press Ctrl+C in Terminal 3 to stop monitor"
    echo "4. 在终端4中按 Ctrl+C 停止日志查看器 | Press Ctrl+C in Terminal 4 to stop viewer"
    echo "或运行清理命令: dora stop dataflow-study-audio-multi.yml"
    echo
}

# 显示故障排除 | Show troubleshooting
show_troubleshooting() {
    echo
    log_step "故障排除 | Troubleshooting"
    echo
    echo -e "${YELLOW}🔧 常见问题 | Common Issues:${NC}"
    echo
    echo "❌ 无音频输出 | No Audio Output:"
    echo "  - 检查系统音量 | Check system volume"
    echo "  - 确认音频设备工作 | Confirm audio device works"
    echo "  - 检查TTS模型是否下载 | Check if TTS models are downloaded"
    echo
    echo "❌ LLM响应慢 | Slow LLM Response:"
    echo "  - 检查API密钥有效性 | Check API key validity"
    echo "  - 考虑使用更快的模型 | Consider using faster models"
    echo "  - 检查网络连接 | Check network connection"
    echo
    echo "❌ 发言顺序混乱 | Speaking Order Chaos:"
    echo "  - 按 'r' 重置 | Press 'r' to reset"
    echo "  - 检查控制器策略配置 | Check controller policy configuration"
    echo
    echo "❌ 内存使用过高 | High Memory Usage:"
    echo "  - 重启系统 | Restart system"
    echo "  - 清理模型缓存 | Clear model cache"
    echo
}

# 主函数 | Main function
main() {
    check_environment
    show_all_commands
    show_cold_start_guide
    show_control_instructions
    show_troubleshooting

    echo
    log_success "启动指南完成 | Startup guide completed"
    echo
    echo -e "${CYAN}🚀 现在请按照上述步骤在4个终端中启动系统 | Now please follow the steps above to start the system in 4 terminals${NC}"
    echo
    echo -e "${GREEN}📖 更多详细信息请查看: GROUP_STUDY_QUICKSTART.md${NC}"
    echo -e "${GREEN}🔗 建议使用手动终端启动以获得最佳控制 | Manual terminal startup recommended for best control${NC}"
    echo
}

# 运行主函数 | Run main function
main "$@"