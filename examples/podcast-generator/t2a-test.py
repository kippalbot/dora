import asyncio
import websockets
import json
import ssl
import subprocess
import os

model = "speech-2.6-hd"
file_format = "mp3"


class StreamAudioPlayer:
    def __init__(self):
        self.mpv_process = None

    def start_mpv(self):
        """Start MPV player process"""
        try:
            mpv_command = ["mpv", "--no-cache", "--no-terminal", "--", "fd://0"]
            self.mpv_process = subprocess.Popen(
                mpv_command,
                stdin=subprocess.PIPE,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            print("MPV player started")
            return True
        except FileNotFoundError:
            print("Error: mpv not found. Please install mpv")
            return False
        except Exception as e:
            print(f"Failed to start mpv: {e}")
            return False

    def play_audio_chunk(self, hex_audio):
        """Play audio chunk"""
        try:
            if self.mpv_process and self.mpv_process.stdin:
                audio_bytes = bytes.fromhex(hex_audio)
                self.mpv_process.stdin.write(audio_bytes)
                self.mpv_process.stdin.flush()
                return True
        except Exception as e:
            print(f"Play failed: {e}")
            return False
        return False

    def stop(self):
        """Stop player"""
        if self.mpv_process:
            if self.mpv_process.stdin and not self.mpv_process.stdin.closed:
                self.mpv_process.stdin.close()
            try:
                self.mpv_process.wait(timeout=20)
            except subprocess.TimeoutExpired:
                self.mpv_process.terminate()


async def establish_connection(api_key):
    """Establish WebSocket connection"""
    url = "wss://api.minimax.io/ws/v1/t2a_v2"
    headers = {"Authorization": f"Bearer {api_key}"}

    ssl_context = ssl.create_default_context()
    ssl_context.check_hostname = False
    ssl_context.verify_mode = ssl.CERT_NONE

    try:
        ws = await websockets.connect(url, additional_headers=headers, ssl=ssl_context)
        connected = json.loads(await ws.recv())
        if connected.get("event") == "connected_success":
            print("Connection successful")
            return ws
        return None
    except Exception as e:
        print(f"Connection failed: {e}")
        return None


async def start_task(websocket):
    """Send task start request"""
    start_msg = {
        "event": "task_start",
        "model": model,
        "voice_setting": {
            "voice_id": "English_expressive_narrator",
            "speed": 1,
            "vol": 1,
            "pitch": 0,
            "english_normalization": False,
        },
        "audio_setting": {
            "sample_rate": 32000,
            "bitrate": 128000,
            "format": file_format,
            "channel": 1,
        },
    }
    await websocket.send(json.dumps(start_msg))
    response = json.loads(await websocket.recv())
    return response.get("event") == "task_started"


async def continue_task_with_stream_play(websocket, text, player):
    """Send continue request and stream play audio"""
    await websocket.send(json.dumps({"event": "task_continue", "text": text}))

    chunk_counter = 1
    total_audio_size = 0
    audio_data = b""

    while True:
        try:
            response = json.loads(await websocket.recv())

            if "data" in response and "audio" in response["data"]:
                audio = response["data"]["audio"]
                if audio:
                    print(f"Playing chunk #{chunk_counter}")
                    audio_bytes = bytes.fromhex(audio)
                    if player.play_audio_chunk(audio):
                        total_audio_size += len(audio_bytes)
                        audio_data += audio_bytes
                        chunk_counter += 1

            if response.get("is_final"):
                print(f"Audio synthesis completed: {chunk_counter - 1} chunks")
                if player.mpv_process and player.mpv_process.stdin:
                    player.mpv_process.stdin.close()

                # Save audio to file
                with open(f"output.{file_format}", "wb") as f:
                    f.write(audio_data)
                print(f"Audio saved as output.{file_format}")

                estimated_duration = total_audio_size * 0.0625 / 1000
                wait_time = max(estimated_duration + 5, 10)
                return wait_time

        except Exception as e:
            print(f"Error: {e}")
            break

    return 10


async def close_connection(websocket):
    """Close connection"""
    if websocket:
        try:
            await websocket.send(json.dumps({"event": "task_finish"}))
            await websocket.close()
        except Exception:
            pass


async def main():
    API_KEY = os.getenv("MINIMAX_API_KEY")
    TEXT = "开源软件的兴起，无疑是二十世纪末以来信息技术领域最深刻的变革之一。它以其开放、协作、共享的精神，汇聚了全球开发者的智慧，催生了无数卓越的技术成果，深刻地影响了软件产业的发展轨迹乃至整个数字经济的构建。然而，在开源运动取得巨大技术成功的同时，其商业化路径却始终面临着复杂而深刻的挑战，形成了一种独特的“开源商业悖论”：一方面，开源模式以其透明性、灵活性和低成本的特性，吸引了广泛的用户和开发者，形成了强大的社区效应和技术生态；另一方面，如何将这种技术优势和社区活力转化为可持续的商业收入，以支持项目的长期维护、迭代创新和团队运营，却一直是困扰众多开源项目的核心难题。成功的开源项目，如Databricks、JetBrains等，背后往往都有强大的商业实体或长期资助者作为支撑，它们通过巧妙的商业模式设计，实现了开源价值与商业回报的良性循环。但对于大多数开源项目而言，找到合适的商业伙伴、构建有效的商业模式，仍然是一个充满不确定性的探索过程。这一悖论在当前以人工智能（AI）为代表的新一代技术浪潮中，显得尤为突出和关键。AI技术的快速发展和广泛应用，催生了对AI平台、工具和框架的巨大需求，而开源模式凭借其在促进技术迭代、降低应用门槛、构建开放生态方面的独特优势，正成为AI创新的重要驱动力。然而，AI开源项目往往技术门槛高、研发投入大、商业化周期长，其对商业支持的需求也更为迫切。因此，深入理解开源商业的内在逻辑，剖析当前面临的挑战，并积极探索有效的解决方案，对于推动新一代AI开源平台的健康发展，具有至关重要的现实意义和战略价值。"

    player = StreamAudioPlayer()

    try:
        if not player.start_mpv():
            return

        ws = await establish_connection(API_KEY)
        if not ws:
            return

        if not await start_task(ws):
            print("Task startup failed")
            return

        wait_time = await continue_task_with_stream_play(ws, TEXT, player)
        await asyncio.sleep(wait_time)

    except Exception as e:
        print(f"Error: {e}")
    finally:
        player.stop()
        if "ws" in locals():
            await close_connection(ws)


if __name__ == "__main__":
    asyncio.run(main())
