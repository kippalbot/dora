#!/bin/bash

# Conference Debate Launcher
# Starts the conference debate with sequential policy

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Dora Conference Debate - Sequential Policy${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""

# Check if API key is set
if [ -z "$ALIBABA_CLOUD_API_KEY" ]; then
    echo -e "${YELLOW}Warning: ALIBABA_CLOUD_API_KEY not set${NC}"
    read -p "Enter your Alibaba Cloud API key: " api_key
    export ALIBABA_CLOUD_API_KEY="$api_key"
fi

# Verify configs exist
echo -e "${BLUE}Checking configuration files...${NC}"
for config in debate_config_maas_llm1.toml debate_config_maas_llm2.toml debate_config_maas_judge.toml; do
    if [ ! -f "$config" ]; then
        echo -e "${YELLOW}Warning: $config not found${NC}"
        echo "You may need to copy from ../llm-client/"
    fi
done
echo "✓ Configuration files present"
echo ""

# Check if binaries exist
echo -e "${BLUE}Checking binaries...${NC}"
for binary in dora-conference-controller dora-conference-bridge dora-maas-client terminal-print; do
    if [ ! -f "../../target/release/$binary" ]; then
        echo -e "${YELLOW}Building $binary...${NC}"
        cd ../..
        cargo build -p "$binary" --release
        cd examples/conference
    fi
done
echo "✓ All binaries present"
echo ""

echo -e "${GREEN}Policy Configuration${NC}"
echo "─────────────────────"
if [ -n "$DORA_POLICY_PATTERN" ]; then
    echo "Pattern: $DORA_POLICY_PATTERN (from environment)"
else
    echo "Pattern: [llm1 → llm2 → judge] (default from dataflow)"
fi
echo ""

echo -e "${GREEN}LLM Configurations${NC}"
echo "───────────────────"
echo "LLM1 (正方): $(grep -o 'system_prompt = ".*"' debate_config_maas_llm1.toml | head -1 | cut -d'"' -f2 | cut -c1-50)..."
echo "LLM2 (反方): $(grep -o 'system_prompt = ".*"' debate_config_maas_llm2.toml | head -1 | cut -d'"' -f2 | cut -c1-50)..."
echo "Judge (主持人): $(grep -o 'system_prompt = ".*"' debate_config_maas_judge.toml | head -1 | cut -d'"' -f2 | cut -c1-50)..."
echo ""

read -p "Press Enter to start debate or Ctrl-C to cancel..."

# Start dora
echo -e "${GREEN}Starting Dora dataflow...${NC}"
echo ""

dora up
dora start dataflow-debate-sequential.yml --attach debate-monitor

# Cleanup on exit
trap 'echo -e "${YELLOW}Shutting down Dora...${NC}"; dora stop; dora destroy --volumes; exit' INT TERM
