# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "openai",
# ]
# ///

from openai import OpenAI
import os

client = OpenAI(
    api_key="dummy_key",
    base_url="http://localhost:8000/v1"
)

text_input = "这是一段测试音频的文本，用来验证系统是否正常工作。在这个快速变化的时代，每一个人都应该有机会接触到最前沿的技术。"

print("1. Sending non-streaming TTS request...")
response = client.audio.speech.create(
    model="tts-1",
    voice="shantianfang",
    input=text_input,
    response_format="mp3"
)
output_filename_non_stream = "output_non_stream.mp3"
# Ignore deprecation warning for the sake of simple testing, or just use content
with open(output_filename_non_stream, "wb") as f:
    f.write(response.content)
print(f"Non-stream saved to {output_filename_non_stream} ({os.path.getsize(output_filename_non_stream)} bytes)\n")

print("2. Sending streaming TTS request...")
# Use with_streaming_response and pass extra_body stream=True
output_filename_stream = "output_stream.mp3"
with client.audio.speech.with_streaming_response.create(
    model="tts-1",
    voice="shantianfang",
    input=text_input,
    response_format="mp3",
    extra_body={"stream": True}
) as stream_response:
    stream_response.stream_to_file(output_filename_stream)

print(f"Stream saved to {output_filename_stream} ({os.path.getsize(output_filename_stream)} bytes)")
print("Success!")
