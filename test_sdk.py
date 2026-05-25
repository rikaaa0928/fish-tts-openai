# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "requests",
# ]
# ///

import os
import sys

from fish_tts_client import FishTTSClient, FishTTSError

BASE_URL = os.getenv("BASE_URL", "http://localhost:8000")
PROXY_API_KEY = os.getenv("PROXY_API_KEY")

passed = 0
failed = 0


def report(name: str, ok: bool, detail: str = ""):
    global passed, failed
    tag = "PASS" if ok else "FAIL"
    if ok:
        passed += 1
    else:
        failed += 1
    print(f"  [{tag}] {name}" + (f" — {detail}" if detail else ""))


# ── 1. Auth ─────────────────────────────────────────────

print("1. Authentication")

if PROXY_API_KEY:
    bad_client = FishTTSClient(base_url=BASE_URL, api_key="invalid_key")
    try:
        bad_client.create_speech(input="test")
        report("reject invalid key", False, "request was accepted")
    except FishTTSError as e:
        report("reject invalid key", e.status_code == 401, f"status={e.status_code}")

    client = FishTTSClient(base_url=BASE_URL, api_key=PROXY_API_KEY)
else:
    print("  (PROXY_API_KEY not set, skipping auth test)")
    client = FishTTSClient(base_url=BASE_URL)

# ── 2. TTS ──────────────────────────────────────────────

print("2. Text-to-Speech")
text_input = "这是一段测试音频，用来验证系统是否正常工作。"

try:
    audio = client.create_speech(input=text_input, voice="shantianfang")
    report("create_speech (bytes)", len(audio) > 0, f"{len(audio)} bytes")
except Exception as e:
    report("create_speech (bytes)", False, str(e))

output_file = "output_test_sdk.mp3"
try:
    client.create_speech(input=text_input, output=output_file)
    size = os.path.getsize(output_file)
    report("create_speech (file)", size > 0, f"{output_file} ({size} bytes)")
except Exception as e:
    report("create_speech (file)", False, str(e))

# ── 3. References ───────────────────────────────────────

print("3. References")

ref_id = "test_sdk_ref"
wav_file = "test_ref.wav"

# generate a minimal WAV so we can test the upload
import struct
def make_test_wav(path: str):
    sr, dur = 16000, 0.5
    n = int(sr * dur)
    data = b"\x00\x00" * n
    with open(path, "wb") as f:
        f.write(b"RIFF")
        f.write(struct.pack("<I", 36 + len(data)))
        f.write(b"WAVE")
        f.write(b"fmt ")
        f.write(struct.pack("<IHHIIHH", 16, 1, 1, sr, sr * 2, 2, 16))
        f.write(b"data")
        f.write(struct.pack("<I", len(data)))
        f.write(data)

make_test_wav(wav_file)

try:
    result = client.add_reference(id=ref_id, text="测试音频文本", audio=wav_file)
    report("add_reference", True, str(result))
except Exception as e:
    report("add_reference", False, str(e))

try:
    refs = client.list_references()
    report("list_references", True, str(refs))
except Exception as e:
    report("list_references", False, str(e))

try:
    result = client.delete_reference(reference_id=ref_id)
    report("delete_reference", True, str(result))
except Exception as e:
    report("delete_reference", False, str(e))

# ── Cleanup & Summary ──────────────────────────────────

for f in [wav_file]:
    if os.path.exists(f):
        os.remove(f)

print(f"\nDone: {passed} passed, {failed} failed")
if failed:
    sys.exit(1)
