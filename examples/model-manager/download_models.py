#!/usr/bin/env python3
"""
Universal Model Downloader
Downloads models from HuggingFace Hub, including PrimeSpeech TTS models and any other HuggingFace models.
"""

import os
import sys
import argparse
from pathlib import Path
from typing import Optional, Dict, List
import json
import fnmatch
import subprocess
import shutil

# Progress bar imports
try:
    from tqdm import tqdm
except ImportError:
    print("Installing tqdm for progress bars...")
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "tqdm"])
    from tqdm import tqdm

# HuggingFace Hub
try:
    from huggingface_hub import snapshot_download, hf_hub_download, list_repo_files, scan_cache_dir
    from huggingface_hub.utils import RepositoryNotFoundError
    HF_AVAILABLE = True
except ImportError:
    print("Installing huggingface-hub...")
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "huggingface-hub"])
    from huggingface_hub import snapshot_download, hf_hub_download, list_repo_files, scan_cache_dir
    from huggingface_hub.utils import RepositoryNotFoundError
    HF_AVAILABLE = True

# Define voice configurations based on actual repository structure
VOICE_CONFIGS = {
    "Doubao": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/doubao-mixed.ckpt",
        "sovits_weights": "SoVITS_weights/doubao-mixed.pth",
        "reference_audio": "ref_audios/doubao_ref_mix_new.wav",
        "text_lang": "zh"
    },
    "Luo Xiang": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/luoxiang_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/luoxiang_best_sovits.pth",
        "reference_audio": "ref_audios/luoxiang_ref.wav",
        "text_lang": "zh"
    },
    "Yang Mi": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/yangmi_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/yangmi_best_sovits.pth",
        "reference_audio": "ref_audios/yangmi_ref.wav",
        "text_lang": "zh"
    },
    "Zhou Jielun": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/zhoujielun_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/zhoujielun_best_sovits.pth",
        "reference_audio": "ref_audios/zhoujielun_ref.wav",
        "text_lang": "zh"
    },
    "Ma Yun": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/mayun_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/mayun_best_sovits.pth",
        "reference_audio": "ref_audios/mayun_ref.wav",
        "text_lang": "zh"
    },
    "Maple": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/maple_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/maple_best_sovits.pth",
        "reference_audio": "ref_audios/maple_ref.wav",
        "text_lang": "en"
    },
    "Cove": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/cove_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/cove_best_sovits.pth",
        "reference_audio": "ref_audios/cove_ref.wav",
        "text_lang": "en"
    },
    "BYS": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/bys_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/bys_best_sovits.pth",
        "reference_audio": "ref_audios/bys_ref.wav",
        "text_lang": "zh"
    },
    "Ellen": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/ellen_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/ellen_best_sovits.pth",
        "reference_audio": "ref_audios/ellen_ref.wav",
        "text_lang": "en"
    },
    "Juniper": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/juniper_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/juniper_best_sovits.pth",
        "reference_audio": "ref_audios/juniper_ref.wav",
        "text_lang": "en"
    },
    "Ma Baoguo": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/mabaoguo_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/mabaoguo_best_sovits.pth",
        "reference_audio": "ref_audios/mabaoguo_ref.wav",
        "text_lang": "zh"
    },
    "Shen Yi": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/shenyi_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/shenyi_best_sovits.pth",
        "reference_audio": "ref_audios/shenyi_ref.wav",
        "text_lang": "zh"
    },
    "Trump": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/trump_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/trump_best_sovits.pth",
        "reference_audio": "ref_audios/trump_ref.wav",
        "text_lang": "en"
    },
    "Daniu": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/dnz_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/dnz_best_sovits.pth",
        "reference_audio": "ref_audios/dnz_ref.wav",
        "text_lang": "zh"
    },
    "Yifan": {
        "repository": "MoYoYoTech/tone-models",
        "gpt_weights": "GPT_weights/yfc_best_gpt.ckpt",
        "sovits_weights": "SoVITS_weights/yfc_best_sovits.pth",
        "reference_audio": "ref_audios/yfc_ref.wav",
        "text_lang": "zh"
    }
}

# Kokoro TTS configuration
KOKORO_DEFAULT_REPO = "hexgrad/Kokoro-82M"  # CPU backend (PyTorch)
KOKORO_MLX_REPO = "prince-canuma/Kokoro-82M"  # MLX backend (Apple Silicon GPU)
KOKORO_MODEL_FILES = [
    "config.json",
    "kokoro-v1_0.pth",
]


def get_kokoro_models_dir() -> Path:
    """Default storage location for Kokoro CPU models."""
    kokoro_dir = os.getenv("KOKORO_MODEL_DIR")
    if kokoro_dir:
        return Path(kokoro_dir)
    return Path.home() / ".dora" / "models" / "kokoro"


def get_kokoro_mlx_models_dir() -> Path:
    """Default storage location for Kokoro MLX models."""
    kokoro_mlx_dir = os.getenv("KOKORO_MLX_MODEL_DIR")
    if kokoro_mlx_dir:
        return Path(kokoro_mlx_dir)
    return Path.home() / ".dora" / "models" / "kokoro-mlx"


def download_kokoro_base(models_dir: Path, repo_id: str = KOKORO_DEFAULT_REPO) -> bool:
    """Download Kokoro base model files (config + checkpoint)."""
    if not HF_AVAILABLE:
        print("❌ huggingface-hub is required to download Kokoro models")
        return False

    models_dir = Path(models_dir)
    models_dir.mkdir(parents=True, exist_ok=True)

    success = True
    print("\n📥 Downloading Kokoro base files")
    for filename in KOKORO_MODEL_FILES:
        target_path = models_dir / filename
        if target_path.exists():
            print(f"   ✓ {filename} (already present)")
            continue
        try:
            print(f"   ⬇️  {filename}")
            hf_hub_download(
                repo_id=repo_id,
                filename=filename,
                local_dir=str(models_dir),
                local_dir_use_symlinks=False,
            )
            print(f"   ✓ Saved to {target_path}")
        except Exception as exc:
            print(f"   ❌ Failed to download {filename}: {exc}")
            success = False

    # Keep HuggingFace cache in sync so the repo is available offline later
    try:
        print(f"   ↻ Updating HuggingFace cache for {repo_id}")
        snapshot_download(repo_id=repo_id, resume_download=True)
        print("   ✓ HuggingFace cache updated")
    except Exception as exc:
        print(f"   ⚠️ Could not refresh HuggingFace cache: {exc}")

    return success


def get_available_kokoro_voices(repo_id: str = KOKORO_DEFAULT_REPO) -> List[str]:
    """Return the list of available Kokoro voice embeddings in the repo."""
    if not HF_AVAILABLE:
        return []
    try:
        files = list_repo_files(repo_id)
    except Exception as exc:
        print(f"❌ Unable to list Kokoro voices: {exc}")
        return []
    voices = sorted(
        {
            Path(f).name
            for f in files
            if f.startswith("voices/") and f.endswith(".pt")
        }
    )
    return voices


def download_kokoro_voices(
    voice: str,
    models_dir: Path,
    repo_id: str = KOKORO_DEFAULT_REPO,
) -> bool:
    """Download one or more Kokoro voice embeddings."""
    if not HF_AVAILABLE:
        print("❌ huggingface-hub is required to download Kokoro voices")
        return False

    available = get_available_kokoro_voices(repo_id)
    if not available:
        print("⚠️ No Kokoro voices discovered; cannot download")
        return False

    if voice.lower() == "all":
        requested = available
    else:
        requested = []
        for item in voice.split(","):
            normalized = item.strip()
            if not normalized:
                continue
            if not normalized.endswith(".pt"):
                normalized = f"{normalized}.pt"
            if normalized not in available:
                print(f"❌ Voice '{normalized}' not found in {repo_id}")
                print(f"   Available voices: {', '.join(available)}")
                return False
            requested.append(normalized)

    if not requested:
        print("⚠️ No Kokoro voices selected for download")
        return False

    models_dir = Path(models_dir)
    models_dir.mkdir(parents=True, exist_ok=True)
    (models_dir / "voices").mkdir(parents=True, exist_ok=True)

    success = True
    print("\n📥 Downloading Kokoro voices")
    for voice_file in requested:
        filename = f"voices/{voice_file}"
        target_path = models_dir / filename
        if target_path.exists():
            print(f"   ✓ {voice_file} (already present)")
            continue
        try:
            print(f"   ⬇️  {voice_file}")
            hf_hub_download(
                repo_id=repo_id,
                filename=filename,
                local_dir=str(models_dir),
                local_dir_use_symlinks=False,
            )
            print(f"   ✓ Saved to {target_path}")
        except Exception as exc:
            print(f"   ❌ Failed to download {voice_file}: {exc}")
            success = False
    return success


def download_kokoro_package(models_dir: Path, repo_id: str = KOKORO_DEFAULT_REPO) -> bool:
    """Download Kokoro base files and all voices."""
    print("\n📦 Downloading Kokoro TTS package")
    print(f"   Repository: {repo_id}")
    base_ok = download_kokoro_base(models_dir, repo_id=repo_id)
    voices_ok = download_kokoro_voices("all", models_dir, repo_id=repo_id)
    return base_ok and voices_ok


def download_kokoro_mlx(models_dir: Path) -> bool:
    """Download MLX-optimized Kokoro model (Apple Silicon GPU).

    Note: MLX uses prince-canuma/Kokoro-82M which is different from the CPU version.
    The MLX model is automatically cached by HuggingFace Hub and used by mlx-audio.
    """
    print("\n📥 Downloading Kokoro MLX (Apple Silicon GPU acceleration)")
    print(f"   Repository: {KOKORO_MLX_REPO}")
    print(f"   Backend: MLX (Metal GPU)")
    print(f"   Platform: macOS Apple Silicon only")
    print(f"   Note: This is a DIFFERENT model than the CPU version (hexgrad/Kokoro-82M)")

    try:
        print("\n   Downloading MLX-optimized Kokoro model...")
        print(f"   This will cache the model in HuggingFace cache directory")

        # Download to HuggingFace cache (mlx-audio will use it from there)
        downloaded_path = snapshot_download(
            repo_id=KOKORO_MLX_REPO,
            resume_download=True
        )

        print(f"   ✅ MLX Kokoro model downloaded successfully")
        print(f"   Location: {downloaded_path}")
        print(f"   Note: mlx-audio will automatically use this cached model")
        print(f"   Model name to use: prince-canuma/Kokoro-82M")

        return True
    except Exception as e:
        print(f"   ❌ Failed to download MLX Kokoro model: {e}")
        return False


def remove_kokoro_base(models_dir: Path) -> bool:
    """Remove Kokoro base files."""
    models_dir = Path(models_dir)
    removed_any = False
    for filename in KOKORO_MODEL_FILES:
        path = models_dir / filename
        if path.exists():
            try:
                path.unlink()
                print(f"   🗑️ Removed {path}")
                removed_any = True
            except Exception as exc:
                print(f"   ❌ Failed to remove {path}: {exc}")
    if not removed_any:
        print("   (No Kokoro base files found to remove)")
    else:
        print("   ✓ Kokoro base files removed")

    # Also drop the HuggingFace cache snapshot to free space
    try:
        cache_removed = remove_huggingface_model(KOKORO_DEFAULT_REPO)
        if cache_removed:
            print(f"   🗑️ Removed HuggingFace cache for {KOKORO_DEFAULT_REPO}")
    except Exception as exc:
        print(f"   ⚠️ Failed to remove HuggingFace cache: {exc}")
    return True


def remove_kokoro_voices(voice: str, models_dir: Path) -> bool:
    """Remove Kokoro voice files."""
    voices_dir = Path(models_dir) / "voices"
    if not voices_dir.exists():
        print("   (No Kokoro voices directory found)")
        return True

    if voice.lower() == "all":
        try:
            shutil.rmtree(voices_dir)
            print(f"   🗑️ Removed {voices_dir}")
        except Exception as exc:
            print(f"   ❌ Failed to remove {voices_dir}: {exc}")
            return False
        return True

    success = True
    for item in voice.split(","):
        normalized = item.strip()
        if not normalized:
            continue
        if not normalized.endswith(".pt"):
            normalized = f"{normalized}.pt"
        file_path = voices_dir / normalized
        if file_path.exists():
            try:
                file_path.unlink()
                print(f"   🗑️ Removed {file_path}")
            except Exception as exc:
                print(f"   ❌ Failed to remove {file_path}: {exc}")
                success = False
        else:
            print(f"   (Voice file not found: {file_path})")
    return success


def remove_kokoro_package(models_dir: Path) -> bool:
    """Remove all Kokoro assets."""
    ok_base = remove_kokoro_base(models_dir)
    ok_voices = remove_kokoro_voices("all", models_dir)
    return ok_base and ok_voices


def remove_kokoro_mlx() -> bool:
    """Remove MLX Kokoro model from HuggingFace cache."""
    print("\n🗑️  Removing Kokoro MLX model")
    print(f"   Repository: {KOKORO_MLX_REPO}")

    return remove_huggingface_model(KOKORO_MLX_REPO)


def list_local_kokoro_voices(models_dir: Path) -> Dict[str, float]:
    """Return mapping of local Kokoro voice filenames to size in MB."""
    voices_dir = Path(models_dir) / "voices"
    if not voices_dir.exists():
        return {}
    voices = {}
    for file_path in sorted(voices_dir.glob("*.pt")):
        try:
            size_mb = file_path.stat().st_size / (1024 ** 2)
        except OSError:
            size_mb = 0.0
        voices[file_path.stem] = size_mb
    return voices

# Standalone helper functions for PrimeSpeech models
def get_primespeech_models_dir():
    """Get the PrimeSpeech models directory."""
    primespeech_model_dir = os.getenv("PRIMESPEECH_MODEL_DIR")
    if primespeech_model_dir:
        return Path(primespeech_model_dir)
    else:
        return Path.home() / ".dora" / "models" / "primespeech"

def check_voice_downloaded(voice_name: str, models_dir: Path) -> tuple[bool, float]:
    """Check if a voice is downloaded and get its size.
    
    Returns:
        (is_downloaded, size_in_mb)
    """
    if voice_name not in VOICE_CONFIGS:
        return False, 0
    
    # Check using simplified naming convention
    voice_lower = voice_name.lower().replace(" ", "").replace("_", "")
    
    # Check in the directory structure used by the actual downloads
    gpt_weights_dir = models_dir / "moyoyo" / "GPT_weights"
    sovits_weights_dir = models_dir / "moyoyo" / "SoVITS_weights"
    ref_audio_dir = models_dir / "moyoyo" / "ref_audios"
    
    # Look for files with the voice name
    gpt_file = None
    sovits_file = None
    ref_file = None
    
    # Check GPT weights
    if gpt_weights_dir.exists():
        for f in gpt_weights_dir.glob("*.ckpt"):
            if voice_lower in f.name.lower().replace("_", ""):
                gpt_file = f
                break
    
    # Check SoVITS weights
    if sovits_weights_dir.exists():
        for f in sovits_weights_dir.glob("*.pth"):
            if voice_lower in f.name.lower().replace("_", ""):
                sovits_file = f
                break
    
    # Check reference audio
    if ref_audio_dir.exists():
        for f in ref_audio_dir.glob("*.wav"):
            if voice_lower in f.name.lower().replace("_", ""):
                ref_file = f
                break
    
    # Also check the original path structure from VOICE_CONFIGS
    if not (gpt_file and sovits_file):
        config = VOICE_CONFIGS[voice_name]
        moyoyo_dir = models_dir / "moyoyo"
        
        # Check if all required files exist at expected paths
        gpt_path = moyoyo_dir / config.get("gpt_weights", "")
        sovits_path = moyoyo_dir / config.get("sovits_weights", "")
        ref_path = moyoyo_dir / config.get("reference_audio", "")
        
        if gpt_path.exists():
            gpt_file = gpt_path
        if sovits_path.exists():
            sovits_file = sovits_path
        if ref_path.exists():
            ref_file = ref_path
    
    # Calculate total size if we have at least GPT and SoVITS files
    if gpt_file and sovits_file:
        expected_ref = VOICE_CONFIGS[voice_name].get("reference_audio")
        # Require the configured reference audio when defined
        if expected_ref:
            ref_path = models_dir / "moyoyo" / expected_ref
            if ref_file is None and not ref_path.exists():
                return False, 0
            ref_file = ref_file or ref_path

        total_size = gpt_file.stat().st_size + sovits_file.stat().st_size
        if ref_file and ref_file.exists():
            total_size += ref_file.stat().st_size
        return True, total_size / (1024**2)  # Convert to MB
    
    return False, 0

def list_downloaded_voices(models_dir: Path) -> dict:
    """List all downloaded voices.
    
    Returns:
        Dictionary with voice names as keys and metadata as values
    """
    downloaded = {}
    for voice_name in VOICE_CONFIGS:
        is_downloaded, size_mb = check_voice_downloaded(voice_name, models_dir)
        if is_downloaded:
            downloaded[voice_name] = {
                "size_mb": size_mb,
                "repository": VOICE_CONFIGS[voice_name]["repository"]
            }
    return downloaded


def list_downloaded_models():
    """List all downloaded models in HuggingFace cache."""
    print("\n📦 Scanning for downloaded models...")
    print("=" * 60)
    
    # Scan HuggingFace cache
    hf_cache_dir = Path.home() / ".cache" / "huggingface" / "hub"
    models_found = []
    
    if hf_cache_dir.exists():
        print(f"\n📁 HuggingFace Cache: {hf_cache_dir}")
        print("-" * 60)
        
        # Try using scan_cache_dir first
        try:
            cache_info = scan_cache_dir(hf_cache_dir)
            
            if cache_info.repos:
                for repo in sorted(cache_info.repos, key=lambda x: x.repo_id):
                    size_gb = repo.size_on_disk / (1024**3)
                    print(f"  📦 {repo.repo_id:40} {size_gb:8.2f} GB")
                    models_found.append(repo.repo_id)
                
        except Exception as e:
            print(f"  Note: HF scan failed ({e}), using directory scan...")
        
        # Also do a direct directory scan for models not detected by scan_cache_dir
        for item in hf_cache_dir.iterdir():
            if item.is_dir() and not item.name.startswith('.'):
                # Determine the repo ID from directory name
                if item.name.startswith("models--"):
                    # Old format: models--org--model
                    repo_id = item.name[8:].replace("--", "/")  # Remove "models--" prefix
                else:
                    # New format: org--model
                    repo_id = item.name.replace("--", "/")
                
                # Skip if already found by scan_cache_dir
                if repo_id not in models_found and not any(repo_id in found for found in models_found):
                    # Calculate size
                    try:
                        size = sum(f.stat().st_size for f in item.rglob("*") if f.is_file())
                        size_gb = size / (1024**3)
                        if size_gb > 0.001:  # Only show if it has meaningful content (>1MB)
                            print(f"  📦 {repo_id:40} {size_gb:8.2f} GB")
                            models_found.append(repo_id)
                    except:
                        pass
        
        if not models_found:
            print("  No models found in HuggingFace cache")
    else:
        print(f"  HuggingFace cache not found at {hf_cache_dir}")
    
    # Also check common model directories
    other_dirs = [
        Path.home() / ".dora" / "models",
        Path.home() / "models",
    ]
    
    for model_dir in other_dirs:
        if model_dir.exists():
            print(f"\n📁 {model_dir}")
            print("-" * 60)
            
            # Special handling for FunASR models
            funasr_dir = model_dir / "asr" / "funasr"
            if funasr_dir.exists():
                funasr_models = []
                for model_path in funasr_dir.iterdir():
                    if model_path.is_dir() and not model_path.name.startswith('.'):
                        size = sum(f.stat().st_size for f in model_path.rglob("*") if f.is_file())
                        if size > 0:
                            size_gb = size / (1024**3)
                            funasr_models.append((model_path.name, size_gb))
                
                if funasr_models:
                    for model_name, size_gb in funasr_models:
                        print(f"  📦 {'asr/funasr/' + model_name:40} {size_gb:8.2f} GB   (FunASR)")
            
            # Look for other model files
            model_files = list(model_dir.glob("**/*.safetensors")) + \
                         list(model_dir.glob("**/*.gguf")) + \
                         list(model_dir.glob("**/*.bin")) + \
                         list(model_dir.glob("**/*.onnx")) + \
                         list(model_dir.glob("**/*.pth")) + \
                         list(model_dir.glob("**/*.ckpt"))
            
            if model_files:
                # Group by parent directory
                model_dirs = {}
                for f in model_files:
                    parent = f.parent
                    # Skip FunASR directories as we handle them separately
                    if "funasr" in str(parent):
                        continue
                    if parent not in model_dirs:
                        model_dirs[parent] = []
                    model_dirs[parent].append(f)
                
                for dir_path, files in sorted(model_dirs.items()):
                    total_size = sum(f.stat().st_size for f in files) / (1024**3)
                    rel_path = dir_path.relative_to(model_dir)
                    print(f"  📦 {str(rel_path):40} {total_size:8.2f} GB   ({len(files)} files)")
            elif not funasr_models:  # Only show "no files" if we didn't find FunASR either
                print(f"  No model files found")
    
    # Count total unique models across all locations
    all_model_count = len(set(models_found))  # HuggingFace models
    
    # Add count from other directories (approximation based on subdirs with model files)
    for model_dir in other_dirs:
        if model_dir.exists():
            model_subdirs = set()
            for f in model_dir.glob("**/*.bin"):
                model_subdirs.add(f.parent)
            for f in model_dir.glob("**/*.safetensors"):
                model_subdirs.add(f.parent)
            for f in model_dir.glob("**/*.gguf"):
                model_subdirs.add(f.parent)
            for f in model_dir.glob("**/*.pth"):
                model_subdirs.add(f.parent)
            for f in model_dir.glob("**/*.ckpt"):
                model_subdirs.add(f.parent)
            all_model_count += len(model_subdirs)
    
    print("\n" + "=" * 60)
    print(f"Total unique models/voices: {all_model_count}")
    return models_found


def download_huggingface_model(repo_id: str, local_dir: Optional[Path] = None, 
                               patterns: Optional[List[str]] = None,
                               revision: str = "main") -> bool:
    """Download any model from HuggingFace Hub.
    
    Args:
        repo_id: Repository ID (e.g., 'mlx-community/gemma-3-12b-it-4bit')
        local_dir: Local directory to save model (default: ~/.cache/huggingface/hub/repo_name)
        patterns: File patterns to download (e.g., ['*.safetensors', '*.json'])
        revision: Git revision to download
        
    Returns:
        True if successful, False otherwise
    """
    print(f"\n📥 Downloading HuggingFace model: {repo_id}")
    
    # Determine local directory
    if local_dir is None:
        repo_name = repo_id.replace("/", "--")
        local_dir = Path.home() / ".cache" / "huggingface" / "hub" / repo_name
    
    print(f"   Destination: {local_dir}")
    
    try:
        # List files in repository
        print("   Fetching file list...")
        files = list_repo_files(repo_id, revision=revision)
        
        # Filter files if patterns provided
        if patterns:
            filtered_files = []
            for file in files:
                for pattern in patterns:
                    if fnmatch.fnmatch(file, pattern):
                        filtered_files.append(file)
                        break
            files = filtered_files
            print(f"   Files to download: {len(files)} (filtered)")
        else:
            print(f"   Files to download: {len(files)}")
        
        # Show file preview
        if len(files) > 10:
            print(f"   First 10 files: {files[:10]}")
            print(f"   ... and {len(files) - 10} more")
        else:
            for f in files:
                print(f"   - {f}")
        
        # Download using snapshot_download
        print("\n⏳ Starting download...")
        downloaded_path = snapshot_download(
            repo_id=repo_id,
            revision=revision,
            local_dir=str(local_dir),
            local_dir_use_symlinks=False,
            allow_patterns=patterns,
            resume_download=True,
            max_workers=4
        )
        
        print(f"✅ Model downloaded successfully to: {downloaded_path}")
        
        # Calculate total size
        total_size = 0
        for root, dirs, filenames in os.walk(local_dir):
            for filename in filenames:
                if not filename.startswith('.'):
                    file_path = Path(root) / filename
                    if file_path.exists():
                        total_size += file_path.stat().st_size
        
        size_gb = total_size / (1024**3)
        print(f"   Total size: {size_gb:.2f} GB")
        
        return True
        
    except RepositoryNotFoundError:
        print(f"❌ Repository not found: {repo_id}")
        print("   Please check the repository name.")
        return False
    except Exception as e:
        print(f"❌ Error downloading model: {e}")
        return False


def download_funasr_models(models_dir: Optional[Path] = None):
    """Download FunASR models from ModelScope for Chinese ASR."""
    print("\n📥 Downloading FunASR models for Chinese ASR")
    print("   Type: FunASR (Paraformer + Punctuation)")
    print("   Source: ModelScope")
    
    # Default ASR models directory
    if models_dir is None:
        asr_models_dir = os.getenv("ASR_MODELS_DIR")
        if asr_models_dir:
            models_dir = Path(asr_models_dir)
        else:
            models_dir = Path.home() / ".dora" / "models" / "asr"
    
    funasr_dir = models_dir / "funasr"
    funasr_dir.mkdir(parents=True, exist_ok=True)
    
    print(f"   Destination: {funasr_dir}")
    
    # Check and install Git LFS if needed
    def check_and_install_git_lfs():
        """Check if Git LFS is installed and install if missing."""
        try:
            result = subprocess.run(["git", "lfs", "version"], capture_output=True, text=True)
            if result.returncode == 0:
                return True
        except FileNotFoundError:
            pass
        
        print("   ⚠️  Git LFS not found. Installing...")
        try:
            # Try to install git-lfs
            if sys.platform == "linux":
                subprocess.run(["sudo", "apt-get", "update"], capture_output=True)
                subprocess.run(["sudo", "apt-get", "install", "-y", "git-lfs"], capture_output=True)
            elif sys.platform == "darwin":
                subprocess.run(["brew", "install", "git-lfs"], capture_output=True)
            else:
                print("   Please install Git LFS manually")
                return False
            
            # Initialize Git LFS
            subprocess.run(["git", "lfs", "install"], capture_output=True)
            return True
        except Exception as e:
            print(f"   Could not install Git LFS automatically: {e}")
            return False
    
    # FunASR models to download
    funasr_models = [
        {
            "name": "ASR Model (Paraformer)",
            "repo_id": "damo/speech_seaco_paraformer_large_asr_nat-zh-cn-16k-common-vocab8404-pytorch",
            "local_name": "speech_seaco_paraformer_large_asr_nat-zh-cn-16k-common-vocab8404-pytorch"
        },
        {
            "name": "Punctuation Model",  
            "repo_id": "damo/punc_ct-transformer_cn-en-common-vocab471067-large",
            "local_name": "punc_ct-transformer_cn-en-common-vocab471067-large"
        }
    ]
    
    downloaded = 0
    for model in funasr_models:
        model_path = funasr_dir / model["local_name"]
        
        if model_path.exists():
            # Check if model weights are actually downloaded (not just LFS pointers)
            model_pt_path = model_path / "model.pt"
            if model_pt_path.exists():
                size_mb = model_pt_path.stat().st_size / (1024**2)
                if size_mb > 1:  # Real model file should be > 1MB
                    print(f"   ✓ {model['name']} already exists ({size_mb:.1f} MB)")
                    downloaded += 1
                    continue
                else:
                    print(f"   ⚠️  {model['name']} exists but model.pt is only {size_mb:.3f} MB (LFS pointer)")
                    print("      Will download actual model weights...")
        
        print(f"   ⏳ Downloading {model['name']}...")
        
        # Try using git clone (ModelScope)
        try:
            import subprocess
            
            # Check Git LFS
            has_lfs = check_and_install_git_lfs()
            
            if model_path.exists() and (model_path / ".git").exists():
                # Repository exists, just need to pull LFS files
                print(f"      Repository exists, pulling LFS files...")
                if has_lfs:
                    subprocess.run(["git", "lfs", "install"], cwd=str(model_path), capture_output=True)
                    result = subprocess.run(["git", "lfs", "pull"], cwd=str(model_path), capture_output=True, text=True)
                    if result.returncode == 0:
                        print(f"   ✅ Downloaded {model['name']} weights")
                        downloaded += 1
                        continue
                    else:
                        print(f"      LFS pull failed: {result.stderr}")
                        print("      Trying full clone...")
                        shutil.rmtree(model_path)
            
            # Clone the repository
            cmd = [
                "git", "clone", 
                f"https://modelscope.cn/models/{model['repo_id']}.git",
                str(model_path)
            ]
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
            
            if result.returncode == 0:
                # After clone, ensure LFS files are downloaded
                if has_lfs and (model_path / ".git").exists():
                    subprocess.run(["git", "lfs", "install"], cwd=str(model_path), capture_output=True)
                    subprocess.run(["git", "lfs", "pull"], cwd=str(model_path), capture_output=True)
                
                # Verify model.pt is downloaded
                model_pt_path = model_path / "model.pt"
                if model_pt_path.exists():
                    size_mb = model_pt_path.stat().st_size / (1024**2)
                    if size_mb > 1:
                        print(f"   ✅ Downloaded {model['name']} ({size_mb:.1f} MB)")
                        downloaded += 1
                    else:
                        print(f"   ⚠️  {model['name']} cloned but model.pt is only {size_mb:.3f} MB")
                        print("      Model weights may not be fully downloaded. Try running:")
                        print(f"      cd {model_path} && git lfs pull")
                else:
                    print(f"   ⚠️  {model['name']} cloned but model.pt not found")
            else:
                print(f"   ❌ Failed to download {model['name']}: {result.stderr}")
                
        except subprocess.TimeoutExpired:
            print(f"   ❌ Download timeout for {model['name']}")
        except FileNotFoundError:
            print("   ❌ Git not installed. Please install git first.")
            print("      macOS: brew install git git-lfs")
            print("      Linux: sudo apt-get install git git-lfs")
        except Exception as e:
            print(f"   ❌ Error downloading {model['name']}: {e}")
    
    if downloaded == len(funasr_models):
        print("✅ All FunASR models downloaded successfully!")
        print(f"   Location: {funasr_dir}")
        print("\n   To use FunASR in ASR node:")
        print("     env:")
        print("       ASR_ENGINE: funasr")
        print(f"       ASR_MODELS_DIR: {models_dir}")
        return True
    else:
        print(f"⚠️  Downloaded {downloaded}/{len(funasr_models)} FunASR models")
        if downloaded == 0:
            print("\n   Manual download instructions:")
            print("   1. Visit https://modelscope.cn/models")
            print("   2. Search for the models above")
            print(f"   3. Download and extract to: {funasr_dir}")
        return downloaded > 0


def download_g2pw_model(models_dir: Path = None):
    """Download G2PW model for Chinese text-to-phoneme conversion."""
    print("\n📥 Downloading G2PW model for Chinese TTS")
    print("   Type: G2PW (Grapheme-to-Phoneme for Chinese)")
    print("   Source: HuggingFace (alextomcat/G2PWModel)")
    
    # Determine target directory in models folder
    if models_dir is None:
        # Use PRIMESPEECH_MODEL_DIR if set, otherwise default
        primespeech_model_dir = os.getenv("PRIMESPEECH_MODEL_DIR")
        if primespeech_model_dir:
            models_dir = Path(primespeech_model_dir)
        else:
            models_dir = Path.home() / ".dora" / "models" / "primespeech"
    
    # G2PW goes in models directory
    g2pw_dir = models_dir / "G2PWModel"
    
    print(f"   Destination: {g2pw_dir}")
    
    # Check if already exists
    if g2pw_dir.exists() and (g2pw_dir / "g2pW.onnx").exists():
        size_mb = (g2pw_dir / "g2pW.onnx").stat().st_size / (1024**2)
        print(f"   ✓ G2PW model already exists ({size_mb:.1f} MB)")
        return True
    
    # Create directory
    g2pw_dir.mkdir(parents=True, exist_ok=True)
    
    # Download from HuggingFace
    try:
        print("   ⏳ Downloading complete G2PW model from HuggingFace...")
        
        repo_id = "alextomcat/G2PWModel"
        
        # Check if already has all necessary files
        required_files = ["g2pW.onnx", "config.py", "bert_config.json", "POLYPHONIC_CHARS.txt"]
        if g2pw_dir.exists() and all((g2pw_dir / f).exists() for f in required_files):
            print(f"   ✓ G2PW model already complete with all required files")
            return True
        
        try:
            # Download entire repository from HuggingFace
            print(f"   Downloading all files from {repo_id}...")
            
            # List all files in the repository
            files = list_repo_files(repo_id)
            print(f"   Found {len(files)} files in repository")
            
            # Download all files
            downloaded_count = 0
            for file in files:
                output_path = g2pw_dir / file
                
                # Skip if already exists
                if output_path.exists():
                    print(f"   ✓ {file} already exists")
                    continue
                
                # Create parent directories if needed
                output_path.parent.mkdir(parents=True, exist_ok=True)
                
                # Download the file
                print(f"   Downloading {file}...")
                downloaded_path = hf_hub_download(
                    repo_id=repo_id,
                    filename=file,
                    local_dir=str(g2pw_dir)
                )
                downloaded_count += 1
            
            print(f"   ✅ Downloaded {downloaded_count} new files")
            print(f"   Location: {g2pw_dir}")
            
            # List all files in the directory
            all_files = list(g2pw_dir.rglob("*"))
            file_count = sum(1 for f in all_files if f.is_file())
            print(f"   Total files: {file_count}")
            
            # Show important files
            for important_file in required_files:
                if (g2pw_dir / important_file).exists():
                    size_mb = (g2pw_dir / important_file).stat().st_size / (1024**2)
                    print(f"      • {important_file} ({size_mb:.1f} MB)")
            
            return True
                
        except Exception as e:
            print(f"   ❌ Error downloading from HuggingFace: {e}")
            raise  # Re-raise to trigger fallback
            
    except RepositoryNotFoundError:
        print(f"   ❌ Repository not found: {repo_id}")
        print("   Please check the repository name.")
        return False
    except Exception as e:
        # Fallback to alternative download method for complete package
        print("\n   Trying alternative download from Google Storage for complete package...")
        url = "https://storage.googleapis.com/esun-ai/g2pW/G2PWModel-v2-onnx.zip"
        
        try:
            import requests
            import zipfile
            import io
            
            print(f"   ⏳ Downloading G2PWModel-v2-onnx.zip (~600MB, includes dictionaries)...")
            response = requests.get(url, stream=True)
            response.raise_for_status()
            
            total_size = int(response.headers.get('content-length', 0))
            
            # Download with progress bar
            from tqdm import tqdm
            content = b""
            with tqdm(total=total_size, unit='B', unit_scale=True, desc="   Downloading") as pbar:
                for chunk in response.iter_content(chunk_size=8192):
                    content += chunk
                    pbar.update(len(chunk))
            
            print("   📦 Extracting G2PW model with dictionaries...")
            with zipfile.ZipFile(io.BytesIO(content)) as zip_file:
                # Extract directly to target directory
                zip_file.extractall(g2pw_dir.parent)
                
                # Check if files were extracted to G2PWModel-v2-onnx and move them
                temp_dir = g2pw_dir.parent / "G2PWModel-v2-onnx"
                if temp_dir.exists():
                    import shutil
                    if g2pw_dir.exists():
                        shutil.rmtree(g2pw_dir)
                    shutil.move(str(temp_dir), str(g2pw_dir))
            
            # List extracted files
            extracted_files = list(g2pw_dir.glob("*"))
            if extracted_files:
                print(f"   ✅ G2PW model downloaded with {len(extracted_files)} files!")
                for f in extracted_files[:5]:  # Show first 5 files
                    size_mb = f.stat().st_size / (1024**2) if f.is_file() else 0
                    print(f"      • {f.name} ({size_mb:.1f} MB)")
                if len(extracted_files) > 5:
                    print(f"      ... and {len(extracted_files) - 5} more files")
            
            print(f"   Location: {g2pw_dir}")
            return True
            
        except Exception as fallback_error:
            print(f"   ❌ Fallback download also failed: {fallback_error}")
            print("\n   Manual download instructions:")
            print(f"   1. Visit: https://huggingface.co/alextomcat/G2PWModel")
            print(f"   2. Download g2pW.onnx")
            print(f"   3. Place it in: {g2pw_dir}")
            print("\n   For complete package with dictionaries:")
            print(f"   1. Download: {url}")
            print(f"   2. Extract to: {g2pw_dir}")
            return False


def download_primespeech_base(models_dir: Path):
    """Download PrimeSpeech base models (Chinese Hubert and Roberta) from HuggingFace."""
    print("\n📥 Downloading PrimeSpeech base models")
    print("   Type: primespeech base (Chinese Hubert & Roberta)")
    print("   Source: MoYoYoTech/tone-models")
    
    moyoyo_dir = models_dir / "moyoyo"
    moyoyo_dir.mkdir(parents=True, exist_ok=True)
    
    # Base pretrained model files needed by all voices
    base_files = {
        'chinese-hubert-base/config.json': 'chinese-hubert-base/config.json',
        'chinese-hubert-base/preprocessor_config.json': 'chinese-hubert-base/preprocessor_config.json',
        'chinese-hubert-base/pytorch_model.bin': 'chinese-hubert-base/pytorch_model.bin',
        'chinese-roberta-wwm-ext-large/config.json': 'chinese-roberta-wwm-ext-large/config.json',
        'chinese-roberta-wwm-ext-large/pytorch_model.bin': 'chinese-roberta-wwm-ext-large/pytorch_model.bin',
        'chinese-roberta-wwm-ext-large/tokenizer.json': 'chinese-roberta-wwm-ext-large/tokenizer.json',
    }
    
    downloaded_count = 0
    for filename in base_files.keys():
        output_path = models_dir / "moyoyo" / filename
        if output_path.exists():
            print(f"✓ {filename} already exists")
            downloaded_count += 1
            continue
            
        try:
            print(f"⏳ Downloading {filename}...")
            # Download from HuggingFace
            file_path = hf_hub_download(
                repo_id="MoYoYoTech/tone-models",
                filename=filename,
                local_dir=str(moyoyo_dir)
            )
            print(f"✅ Downloaded {filename}")
            downloaded_count += 1
        except Exception as e:
            print(f"❌ Error downloading {filename}: {e}")
    
    if downloaded_count == len(base_files):
        print("✅ All PrimeSpeech base models downloaded successfully!")
        return True
    else:
        print(f"⚠️  Downloaded {downloaded_count}/{len(base_files)} base model files")
        return downloaded_count > 0


def download_voice_models(voice_name: str, models_dir: Path):
    """Download voice-specific models."""
    if voice_name == "all":
        voices_to_download = list(VOICE_CONFIGS.keys())
    else:
        if voice_name not in VOICE_CONFIGS:
            print(f"Error: Unknown voice '{voice_name}'")
            print(f"Available voices: {', '.join(VOICE_CONFIGS.keys())}")
            return False
        voices_to_download = [voice_name]
    
    print(f"\nVoices to download: {', '.join(voices_to_download)}")
    print("-" * 50)
    
    moyoyo_dir = models_dir / "moyoyo"
    moyoyo_dir.mkdir(parents=True, exist_ok=True)
    
    for voice in voices_to_download:
        voice_config = VOICE_CONFIGS[voice]
        
        print(f"\n[{voice}]")
        
        # Check if already downloaded
        is_downloaded, size_mb = check_voice_downloaded(voice, models_dir)
        if is_downloaded:
            print(f"  ✓ Already downloaded ({size_mb:.1f} MB)")
            continue
        
        print(f"  Downloading from {voice_config['repository']}...")
        try:
            # Download each file
            files_to_download = [
                voice_config.get("gpt_weights"),
                voice_config.get("sovits_weights"),
                voice_config.get("reference_audio")
            ]
            
            for file_path in files_to_download:
                if file_path:
                    hf_hub_download(
                        repo_id=voice_config["repository"],
                        filename=file_path,
                        local_dir=str(moyoyo_dir)
                    )
            
            # Check if successfully downloaded
            is_downloaded, size_mb = check_voice_downloaded(voice, models_dir)
            if is_downloaded:
                print(f"  ✓ Downloaded successfully ({size_mb:.1f} MB)")
            else:
                print(f"  ✗ Failed to download")
        except Exception as e:
            print(f"  ✗ Failed to download: {e}")
    
    return True


def remove_huggingface_model(repo_id: str) -> bool:
    """Remove a HuggingFace model from cache.
    
    Args:
        repo_id: Repository ID to remove (e.g., 'mlx-community/gemma-3-12b-it-4bit')
        
    Returns:
        True if successful, False otherwise
    """
    print(f"\n🗑️  Removing HuggingFace model: {repo_id}")
    
    # Check multiple possible cache locations
    repo_name = repo_id.replace("/", "--")
    possible_locations = [
        Path.home() / ".cache" / "huggingface" / "hub" / repo_name,
        Path.home() / ".cache" / "huggingface" / "hub" / f"models--{repo_name}",
    ]
    
    found = False
    for cache_path in possible_locations:
        if cache_path.exists():
            print(f"   Found at: {cache_path}")
            
            # Calculate size before deletion
            total_size = sum(f.stat().st_size for f in cache_path.rglob("*") if f.is_file())
            size_gb = total_size / (1024**3)
            print(f"   Size: {size_gb:.2f} GB")
            
            # Ask for confirmation
            response = input(f"   Are you sure you want to remove this model? (yes/no): ").lower().strip()
            if response in ['yes', 'y']:
                try:
                    shutil.rmtree(cache_path)
                    print(f"✅ Successfully removed {repo_id}")
                    found = True
                    break
                except Exception as e:
                    print(f"❌ Error removing model: {e}")
                    return False
            else:
                print("   Cancelled.")
                return False
    
    if not found:
        print(f"❌ Model not found in cache: {repo_id}")
        print("   Use --list to see downloaded models")
        return False
    
    return True


def remove_funasr_models() -> bool:
    """Remove FunASR models."""
    print("\n🗑️  Removing FunASR models")
    
    # Check for ASR models directory
    asr_models_dir = os.getenv("ASR_MODELS_DIR")
    if asr_models_dir:
        funasr_dir = Path(asr_models_dir) / "funasr"
    else:
        funasr_dir = Path.home() / ".dora" / "models" / "asr" / "funasr"
    
    if not funasr_dir.exists():
        print(f"❌ FunASR models not found at: {funasr_dir}")
        return False
    
    print(f"   Location: {funasr_dir}")
    
    # List models
    models = []
    for model_dir in funasr_dir.iterdir():
        if model_dir.is_dir() and not model_dir.name.startswith('.'):
            size = sum(f.stat().st_size for f in model_dir.rglob("*") if f.is_file())
            size_mb = size / (1024**2)
            models.append((model_dir.name, model_dir, size_mb))
            print(f"   - {model_dir.name} ({size_mb:.1f} MB)")
    
    if not models:
        print("   No FunASR models found")
        return False
    
    # Ask for confirmation
    response = input(f"\n   Remove all FunASR models? (yes/no): ").lower().strip()
    if response in ['yes', 'y']:
        try:
            shutil.rmtree(funasr_dir)
            print(f"✅ Successfully removed all FunASR models")
            return True
        except Exception as e:
            print(f"❌ Error removing FunASR models: {e}")
            return False
    else:
        print("   Cancelled.")
        return False


def remove_voice_models(voice_name: str, models_dir: Path) -> bool:
    """Remove voice-specific models.
    
    Args:
        voice_name: Name of voice to remove or 'all' for all voices
        models_dir: Models directory
        
    Returns:
        True if successful, False otherwise
    """
    moyoyo_dir = models_dir / "moyoyo"
    
    if voice_name == "all":
        print("\n🗑️  Removing ALL PrimeSpeech voice models")
        
        # List all downloaded voices
        available = list_downloaded_voices(models_dir)
        if not available:
            print("   No voices found to remove")
            return False
        
        print(f"   Found {len(available)} voices:")
        total_size = 0
        for name, metadata in available.items():
            size_mb = metadata.get("size_mb", 0)
            total_size += size_mb
            print(f"   - {name} ({size_mb:.1f} MB)")
        
        print(f"\n   Total size: {total_size:.1f} MB")
        
        # Ask for confirmation
        response = input(f"   Remove ALL voice models? (yes/no): ").lower().strip()
        if response in ['yes', 'y']:
            try:
                if moyoyo_dir.exists():
                    shutil.rmtree(moyoyo_dir)
                print(f"✅ Successfully removed all voice models")
                return True
            except Exception as e:
                print(f"❌ Error removing voice models: {e}")
                return False
        else:
            print("   Cancelled.")
            return False
            
    else:
        # Remove specific voice
        print(f"\n🗑️  Removing voice model: {voice_name}")
        
        if voice_name not in VOICE_CONFIGS:
            print(f"❌ Unknown voice: {voice_name}")
            print(f"   Available voices: {', '.join(VOICE_CONFIGS.keys())}")
            return False
        
        voice_config = VOICE_CONFIGS[voice_name]
        
        # Check if voice is downloaded
        is_downloaded, size_mb = check_voice_downloaded(voice_name, models_dir)
        if not is_downloaded:
            print(f"❌ Voice '{voice_name}' is not downloaded")
            return False
        print(f"   Size: {size_mb:.1f} MB")
        
        # Files to remove
        files_to_remove = [
            moyoyo_dir / voice_config.get("gpt_weights", ""),
            moyoyo_dir / voice_config.get("sovits_weights", ""),
            moyoyo_dir / voice_config.get("reference_audio", "")
        ]
        
        # Ask for confirmation
        response = input(f"   Remove '{voice_name}' voice model? (yes/no): ").lower().strip()
        if response in ['yes', 'y']:
            removed_count = 0
            for file_path in files_to_remove:
                if file_path and file_path.exists():
                    try:
                        file_path.unlink()
                        removed_count += 1
                    except Exception as e:
                        print(f"   Warning: Could not remove {file_path.name}: {e}")
            
            if removed_count > 0:
                print(f"✅ Successfully removed {voice_name} ({removed_count} files)")
                
                # Check if we should also remove base models
                remaining_voices = list_downloaded_voices(models_dir)
                if not remaining_voices:
                    response = input("\n   No voices remaining. Remove base models too? (yes/no): ").lower().strip()
                    if response in ['yes', 'y']:
                        remove_primespeech_base_models(models_dir)
                
                return True
            else:
                print(f"❌ No files removed for {voice_name}")
                return False
        else:
            print("   Cancelled.")
            return False


def remove_g2pw_model(models_dir: Path = None) -> bool:
    """Remove G2PW model.
    
    Args:
        models_dir: Models directory (default: ~/.dora/models/primespeech)
        
    Returns:
        True if successful, False otherwise
    """
    print("\n🗑️  Removing G2PW model")
    
    # Determine models directory
    if models_dir is None:
        primespeech_model_dir = os.getenv("PRIMESPEECH_MODEL_DIR")
        if primespeech_model_dir:
            models_dir = Path(primespeech_model_dir)
        else:
            models_dir = Path.home() / ".dora" / "models" / "primespeech"
    
    g2pw_dir = models_dir / "G2PWModel"
    
    if not g2pw_dir.exists():
        print(f"❌ G2PW model not found at: {g2pw_dir}")
        return False
    
    # Calculate size
    total_size = 0
    for file in g2pw_dir.rglob("*"):
        if file.is_file():
            total_size += file.stat().st_size
    
    size_mb = total_size / (1024**2)
    print(f"   Location: {g2pw_dir}")
    print(f"   Size: {size_mb:.1f} MB")
    
    # Ask for confirmation
    response = input(f"   Remove G2PW model? (yes/no): ").lower().strip()
    if response in ['yes', 'y']:
        try:
            shutil.rmtree(g2pw_dir)
            print(f"✅ Successfully removed G2PW model")
            return True
        except Exception as e:
            print(f"❌ Error removing G2PW model: {e}")
            return False
    else:
        print("   Cancelled.")
        return False


def remove_primespeech_base_models(models_dir: Path) -> bool:
    """Remove PrimeSpeech base models (Chinese Hubert and Roberta).
    
    Args:
        models_dir: Models directory
        
    Returns:
        True if successful, False otherwise
    """
    print("\n🗑️  Removing PrimeSpeech base models")
    
    moyoyo_dir = models_dir / "moyoyo"
    base_dirs = [
        moyoyo_dir / "chinese-hubert-base",
        moyoyo_dir / "chinese-roberta-wwm-ext-large"
    ]
    
    total_size = 0
    for base_dir in base_dirs:
        if base_dir.exists():
            size = sum(f.stat().st_size for f in base_dir.rglob("*") if f.is_file())
            total_size += size
            size_mb = size / (1024**2)
            print(f"   - {base_dir.name} ({size_mb:.1f} MB)")
    
    if total_size == 0:
        print("   No base models found")
        return False
    
    total_mb = total_size / (1024**2)
    print(f"   Total size: {total_mb:.1f} MB")
    
    # Don't ask for confirmation if called from remove_voice_models
    # The user already confirmed
    try:
        for base_dir in base_dirs:
            if base_dir.exists():
                shutil.rmtree(base_dir)
        print(f"✅ Successfully removed base models")
        return True
    except Exception as e:
        print(f"❌ Error removing base models: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(description="Universal Model Downloader - Download and manage models")
    
    # Add --download argument for compatibility
    parser.add_argument(
        "--download",
        type=str,
        help=(
            "Model to download: 'funasr', 'primespeech', 'primespeech-base', 'g2pw', 'kokoro', "
            "'kokoro-base', 'kokoro-voices', 'kokoro-mlx', or a HuggingFace repo ID"
        )
    )
    
    # Add --remove argument
    parser.add_argument(
        "--remove",
        type=str,
        help=(
            "Model to remove: 'funasr', 'g2pw', voice name, 'all-voices', 'primespeech-base', "
            "'kokoro', 'kokoro-mlx', 'kokoro-base', 'kokoro-voices', or HuggingFace repo ID"
        )
    )
    
    # HuggingFace-specific arguments
    parser.add_argument(
        "--hf-dir",
        type=str,
        default=None,
        help="Local directory for HuggingFace model (default: ~/.cache/huggingface/hub/repo_name)"
    )
    
    parser.add_argument(
        "--patterns",
        type=str,
        nargs="+",
        help="File patterns to download (e.g., '*.safetensors' '*.json')"
    )
    
    parser.add_argument(
        "--revision",
        type=str,
        default="main",
        help="Git revision to download (default: main)"
    )
    
    # Original arguments
    parser.add_argument(
        "--voice",
        type=str,
        default=None,
        help=f"Voice to download. Available: all, {', '.join(VOICE_CONFIGS.keys())}"
    )
    parser.add_argument(
        "--models-dir",
        type=str,
        default=None,
        help="Directory to store models (default: ~/.dora/models/primespeech)"
    )
    parser.add_argument(
        "--kokoro-dir",
        type=str,
        default=None,
        help="Directory to store Kokoro models (default: ~/.dora/models/kokoro)"
    )
    parser.add_argument(
        "--kokoro-voice",
        type=str,
        default=None,
        help="Kokoro voice to download (e.g., af_heart or 'all')"
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="List all downloaded models"
    )
    
    parser.add_argument(
        "--list-voices",
        action="store_true",
        help="List available PrimeSpeech voices"
    )
    parser.add_argument(
        "--list-kokoro-voices",
        action="store_true",
        help="List available Kokoro voices"
    )
    
    args = parser.parse_args()
    kokoro_models_dir = Path(args.kokoro_dir) if args.kokoro_dir else get_kokoro_models_dir()
    kokoro_mlx_models_dir = get_kokoro_mlx_models_dir()
    
    # Handle --list (show all downloaded models)
    if args.list:
        list_downloaded_models()
        return
    
    # Handle --list-voices (show PrimeSpeech voices)
    if args.list_voices:
        # Get models directory
        if args.models_dir:
            models_dir = Path(args.models_dir)
        else:
            models_dir = get_primespeech_models_dir()
        
        downloaded_voices = list_downloaded_voices(models_dir)
        
        print("\nPrimeSpeech Voices:")
        print("=" * 60)
        print(f"{'Voice Name':<20} {'Language':<10} {'Status':<12} {'Size'}")
        print("-" * 60)
        
        for voice_name, config in VOICE_CONFIGS.items():
            lang = config.get("text_lang", "unknown")
            
            # Check if voice is downloaded
            if voice_name in downloaded_voices:
                size_mb = downloaded_voices[voice_name].get("size_mb", 0)
                status = "✅ Downloaded"
                size_str = f"{size_mb:.1f} MB"
            else:
                status = "⬇️  Available"
                size_str = "-"
            
            print(f"  {voice_name:<18} {lang:<10} {status:<12} {size_str}")
        
        # Show summary
        print("-" * 60)
        print(f"Downloaded: {len(downloaded_voices)}/{len(VOICE_CONFIGS)} voices")
        
        if len(downloaded_voices) < len(VOICE_CONFIGS):
            print("\nTo download voices:")
            print("  python download_models.py --voice <voice_name>")
            print("  python download_models.py --voice all")
        return

    if args.list_kokoro_voices:
        available = get_available_kokoro_voices()
        if not available:
            print("\n⚠️ Unable to retrieve Kokoro voice list (check network or huggingface-hub installation)")
            return

        print("\nKokoro Voices:")
        print("=" * 60)
        for voice_file in available:
            print(f"  {voice_file[:-3] if voice_file.endswith('.pt') else voice_file}")
        print(f"\nTotal voices: {len(available)}")
        return
    
    # Get models directory for PrimeSpeech operations
    if args.models_dir:
        models_dir = Path(args.models_dir)
    else:
        models_dir = get_primespeech_models_dir()
    
    # Only print for PrimeSpeech operations
    if args.voice or (args.download and args.download == "primespeech-base"):
        print(f"\nPrimeSpeech models directory: {models_dir}")

    if args.kokoro_voice or (args.download and args.download.startswith("kokoro")):
        print(f"\nKokoro models directory: {kokoro_models_dir}")
    
    # Handle --download argument
    if args.download:
        # Check if it's a HuggingFace repo (contains '/')
        if '/' in args.download:
            # It's a HuggingFace repo ID
            local_dir = Path(args.hf_dir) if args.hf_dir else None
            success = download_huggingface_model(
                repo_id=args.download,
                local_dir=local_dir,
                patterns=args.patterns,
                revision=args.revision
            )
            if not success:
                sys.exit(1)
        elif args.download == "funasr":
            # Download FunASR models
            success = download_funasr_models()
            if not success:
                sys.exit(1)
        elif args.download == "primespeech-base":
            success = download_primespeech_base(models_dir)
            if not success:
                sys.exit(1)
        elif args.download == "g2pw":
            # Download G2PW model
            success = download_g2pw_model()
            if not success:
                sys.exit(1)
        elif args.download == "kokoro-base":
            success = download_kokoro_base(kokoro_models_dir)
            if not success:
                sys.exit(1)
        elif args.download == "kokoro-voices":
            success = download_kokoro_voices("all", kokoro_models_dir)
            if not success:
                sys.exit(1)
        elif args.download == "kokoro":
            success = download_kokoro_package(kokoro_models_dir)
            if not success:
                sys.exit(1)
        elif args.download == "kokoro-mlx":
            # MLX uses separate directory and HuggingFace cache
            success = download_kokoro_mlx(kokoro_mlx_models_dir)
            if not success:
                sys.exit(1)
        elif args.download == "primespeech":
            # Download all PrimeSpeech required models (base, G2PW, and all voices)
            print("\n📦 Downloading complete PrimeSpeech package")
            print("=" * 60)
            
            all_success = True
            
            # 1. Download base models (Chinese Hubert & Roberta)
            print("\n[1/3] Downloading PrimeSpeech base models...")
            success = download_primespeech_base(models_dir)
            if not success:
                print("❌ Failed to download base models")
                all_success = False
            
            # 2. Download G2PW model
            print("\n[2/3] Downloading G2PW model...")
            success = download_g2pw_model()
            if not success:
                print("❌ Failed to download G2PW model")
                all_success = False
            
            # 3. Download all voice models
            print("\n[3/3] Downloading all voice models...")
            success = download_voice_models("all", models_dir)
            if not success:
                print("❌ Failed to download voice models")
                all_success = False
            
            if all_success:
                print("\n✅ Successfully downloaded all PrimeSpeech components!")
            else:
                print("\n⚠️ Some components failed to download. Please check errors above.")
                sys.exit(1)
        elif args.download in VOICE_CONFIGS:
            # Download specific voice
            success = download_voice_models(args.download, models_dir)
            if not success:
                sys.exit(1)
        else:
            print(f"❌ Unknown model to download: {args.download}")
            print("   Valid options:")
            print("   - 'funasr' for FunASR models")
            print("   - 'primespeech' for complete PrimeSpeech package (base + G2PW + all voices)")
            print("   - 'primespeech-base' for PrimeSpeech base models only")
            print("   - 'g2pw' for G2PW model only")
            print("   - 'kokoro' for Kokoro base + all voices (CPU backend)")
            print("   - 'kokoro-mlx' for MLX-optimized Kokoro (Apple Silicon GPU)")
            print("   - 'kokoro-base' for Kokoro base files only")
            print("   - 'kokoro-voices' for all Kokoro voices only")
            print(f"   - Voice name: {', '.join(VOICE_CONFIGS.keys())}")
            print("   - HuggingFace repo ID (e.g., 'organization/model')")
            sys.exit(1)
    
    # Handle --remove argument
    elif args.remove:
        # Ensure models_dir is set for voice operations
        if args.models_dir:
            models_dir = Path(args.models_dir)
        else:
            models_dir = get_primespeech_models_dir()
        
        remove_lower = args.remove.lower()

        # Check if it's a HuggingFace repo (contains '/')
        if '/' in args.remove:
            # It's a HuggingFace repo ID
            success = remove_huggingface_model(args.remove)
            if not success:
                sys.exit(1)
        elif remove_lower == "funasr":
            # Remove FunASR models
            success = remove_funasr_models()
            if not success:
                sys.exit(1)
        elif remove_lower == "all-voices":
            # Remove all voice models
            success = remove_voice_models("all", models_dir)
            if not success:
                sys.exit(1)
        elif remove_lower == "g2pw":
            # Remove G2PW model
            success = remove_g2pw_model()
            if not success:
                sys.exit(1)
        elif remove_lower == "primespeech-base":
            # Remove base models
            success = remove_primespeech_base_models(models_dir)
            if not success:
                sys.exit(1)
        elif remove_lower == "kokoro-base":
            success = remove_kokoro_base(kokoro_models_dir)
            if not success:
                sys.exit(1)
        elif remove_lower == "kokoro-voices":
            success = remove_kokoro_voices("all", kokoro_models_dir)
            if not success:
                sys.exit(1)
        elif remove_lower == "kokoro":
            success = remove_kokoro_package(kokoro_models_dir)
            if not success:
                sys.exit(1)
        elif remove_lower == "kokoro-mlx":
            success = remove_kokoro_mlx()
            if not success:
                sys.exit(1)
        elif args.remove in VOICE_CONFIGS:
            # Remove specific voice
            success = remove_voice_models(args.remove, models_dir)
            if not success:
                sys.exit(1)
        else:
            print(f"❌ Unknown model to remove: {args.remove}")
            print("   Valid options:")
            print("   - HuggingFace repo ID (e.g., 'mlx-community/gemma-3-12b-it-4bit')")
            print("   - 'funasr' to remove FunASR models")
            print("   - 'g2pw' to remove G2PW model")
            print("   - 'all-voices' to remove all PrimeSpeech voices")
            print("   - 'primespeech-base' to remove base models")
            print("   - 'kokoro', 'kokoro-mlx', 'kokoro-base', 'kokoro-voices' for Kokoro assets")
            print(f"   - Voice name: {', '.join(VOICE_CONFIGS.keys())}")
            sys.exit(1)
    
    # Handle --voice argument
    elif args.voice:
        success = download_voice_models(args.voice, models_dir)
        if not success:
            sys.exit(1)
    elif args.kokoro_voice:
        success = download_kokoro_voices(args.kokoro_voice, kokoro_models_dir)
        if not success:
            sys.exit(1)
    
    # Default: show help
    else:
        print("\nUsage examples:")
        print("\n  # Download any HuggingFace model:")
        print("  python download_models.py --download mlx-community/gemma-3-12b-it-4bit")
        print("  python download_models.py --download Qwen/Qwen3-8B-MLX-4bit")
        print("")
        print("  # Download with custom directory:")
        print("  python download_models.py --download mlx-community/gemma-3-12b-it-4bit --hf-dir ~/models/gemma")
        print("")
        print("  # Download only specific files:")
        print("  python download_models.py --download mlx-community/gemma-3-12b-it-4bit --patterns '*.safetensors' '*.json'")
        print("")
        print("  # Download FunASR models (Chinese ASR):")
        print("  python download_models.py --download funasr")
        print("")
        print("  # Download G2PW model (Chinese text-to-phoneme for TTS):")
        print("  python download_models.py --download g2pw")
        print("")
        print("  # List downloaded models:")
        print("  python download_models.py --list")
        
        print("\n  # PrimeSpeech TTS models:")
        print("  python download_models.py --download primespeech       # Download all voices")
        print("  python download_models.py --download primespeech-base  # Download base models only")
        print("  python download_models.py --download Doubao           # Download specific voice")
        print("  python download_models.py --voice all                 # Download all voices (alternative)")
        print("  python download_models.py --voice \"Luo Xiang\"         # Download specific voice (alternative)")
        print("  python download_models.py --list-voices               # List available voices")

        print("\n  # Kokoro TTS models:")
        print("  python download_models.py --download kokoro           # CPU backend (hexgrad/Kokoro-82M)")
        print("  python download_models.py --download kokoro-mlx       # MLX backend (prince-canuma/Kokoro-82M)")
        print("  python download_models.py --download kokoro-base      # CPU base files only")
        print("  python download_models.py --download kokoro-voices    # CPU voices only")
        print("  python download_models.py --kokoro-voice af_heart     # Download specific CPU voice")
        print("  python download_models.py --list-kokoro-voices        # List Kokoro voices")
        print("")
        print("  Note: CPU and MLX use DIFFERENT models from different repositories:")
        print("    - CPU:  hexgrad/Kokoro-82M      (PyTorch, ~/.dora/models/kokoro)")
        print("    - MLX:  prince-canuma/Kokoro-82M (Metal GPU, HuggingFace cache)")

        print("\n  # Remove models:")
        print("  python download_models.py --remove mlx-community/gemma-3-12b-it-4bit")
        print("  python download_models.py --remove funasr")
        print("  python download_models.py --remove kokoro           # Remove CPU models")
        print("  python download_models.py --remove kokoro-mlx       # Remove MLX model")
        print("  python download_models.py --remove \"Luo Xiang\"")
        print("  python download_models.py --remove all-voices")
        print("  python download_models.py --remove primespeech-base")
        return
    
    # Only show available voices if we downloaded something
    if args.download or args.voice:
        print("\n" + "=" * 50)
        print("Available voices on disk:")
        print("-" * 50)
        
        available = list_downloaded_voices(models_dir)
        if available:
            for voice_name, metadata in available.items():
                repo = metadata.get("repository", "unknown")
                print(f"  {voice_name:15} - Repository: {repo}")
        else:
            print("  No voices found")

    if (args.download and args.download.startswith("kokoro")) or args.kokoro_voice:
        print("\n" + "=" * 50)
        print("Available Kokoro voices on disk:")
        print("-" * 50)

        kokoro_local = list_local_kokoro_voices(kokoro_models_dir)
        if kokoro_local:
            for voice_name, size_mb in kokoro_local.items():
                print(f"  {voice_name:20} {size_mb:6.1f} MB")
        else:
            print("  No Kokoro voices found")

    print("\nDone!")


if __name__ == "__main__":
    main()
