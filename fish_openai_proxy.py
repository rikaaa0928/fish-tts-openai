# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "fastapi",
#     "uvicorn",
#     "httpx",
# ]
# ///

import os
import httpx
from fastapi import FastAPI, Request, HTTPException, Response
from fastapi.responses import StreamingResponse

app = FastAPI(title="Fish TTS to OpenAI Proxy")

FISH_TTS_URL = os.getenv("FISH_TTS_URL", "http://192.168.0.69:8080/v1/tts")
FISH_TTS_KEY = os.getenv("FISH_TTS_KEY", "key")

@app.post("/v1/audio/speech")
async def create_speech(request: Request):
    try:
        body = await request.json()
    except Exception:
        raise HTTPException(status_code=400, detail="Invalid JSON body")

    text = body.get("input")
    if not text:
        raise HTTPException(status_code=400, detail="Missing 'input' field")

    # Map OpenAI 'voice' to Fish TTS 'reference_id'. Default to shantianfang if not provided.
    voice = body.get("voice", "shantianfang")
    response_format = body.get("response_format", "mp3")
    
    # Optional parameters that can be passed
    chunk_length = body.get("chunk_length", 300)
    temperature = body.get("temperature", 1.0)
    # is_stream = body.get("stream", False)
    is_stream = False

    # Fish TTS payload - removed 'stream' and 'chunk_length' as per requirements
    fish_payload = {
        "text": text,
        "reference_id": voice,
        "format": response_format,
        "chunk_length": chunk_length,
        "temperature": temperature,
    }

    headers = {
        "Authorization": f"Bearer {FISH_TTS_KEY}",
        "Accept": "application/json, audio/*",
        "Content-Type": "application/json"
    }

    client = httpx.AsyncClient(timeout=120.0)
    
    try:
        # Build request to Fish TTS
        req = client.build_request("POST", FISH_TTS_URL, json=fish_payload, headers=headers)
        # We still use stream=True in httpx to read the response efficiently
        resp = await client.send(req, stream=True)
    except Exception as e:
        await client.aclose()
        raise HTTPException(status_code=500, detail=f"Failed to connect to Fish TTS: {str(e)}")

    if resp.status_code != 200:
        await resp.aread()
        text_err = resp.text
        await resp.aclose()
        await client.aclose()
        raise HTTPException(status_code=resp.status_code, detail=f"Fish TTS error: {text_err}")

    media_type = f"audio/{response_format}" if response_format in ["mp3", "opus", "aac", "flac"] else "application/octet-stream"
    if response_format == "pcm":
        media_type = "audio/pcm"

    if is_stream:
        async def stream_audio():
            try:
                async for chunk in resp.aiter_bytes():
                    yield chunk
            finally:
                await resp.aclose()
                await client.aclose()
        return StreamingResponse(stream_audio(), media_type=media_type)
    else:
        try:
            content = await resp.aread()
            return Response(content=content, media_type=media_type)
        finally:
            await resp.aclose()
            await client.aclose()

if __name__ == "__main__":
    import uvicorn
    port = int(os.getenv("PORT", 8000))
    print(f"Starting Fish TTS to OpenAI Proxy on port {port}...")
    uvicorn.run(app, host="0.0.0.0", port=port)
