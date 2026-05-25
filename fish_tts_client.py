"""Fish TTS OpenAI Proxy Python SDK"""

from __future__ import annotations

import os
from pathlib import Path
from typing import BinaryIO, Union

import requests


class FishTTSError(Exception):
    def __init__(self, status_code: int, detail: str):
        self.status_code = status_code
        self.detail = detail
        super().__init__(f"[{status_code}] {detail}")


class FishTTSClient:
    def __init__(self, base_url: str, api_key: str | None = None):
        self.base_url = base_url.rstrip("/")
        self.session = requests.Session()
        if api_key:
            self.session.headers["Authorization"] = f"Bearer {api_key}"

    def _raise_for_error(self, resp: requests.Response) -> None:
        if resp.ok:
            return
        try:
            detail = resp.json().get("detail", resp.text)
        except Exception:
            detail = resp.text
        raise FishTTSError(resp.status_code, detail)

    # ── TTS ──────────────────────────────────────────────

    def create_speech(
        self,
        input: str,
        voice: str = "shantianfang",
        response_format: str = "mp3",
        chunk_length: int = 300,
        temperature: float = 1.0,
        output: str | Path | BinaryIO | None = None,
    ) -> bytes | None:
        """Generate speech audio.

        If *output* is given (file path or writable binary IO), the audio is
        streamed there and ``None`` is returned.  Otherwise the raw bytes are
        returned.
        """
        payload = {
            "input": input,
            "voice": voice,
            "response_format": response_format,
            "chunk_length": chunk_length,
            "temperature": temperature,
        }
        resp = self.session.post(
            f"{self.base_url}/v1/audio/speech",
            json=payload,
            stream=output is not None,
        )
        self._raise_for_error(resp)

        if output is None:
            return resp.content

        fp: BinaryIO
        need_close = False
        if isinstance(output, (str, Path)):
            fp = open(output, "wb")
            need_close = True
        else:
            fp = output
        try:
            for chunk in resp.iter_content(chunk_size=8192):
                fp.write(chunk)
        finally:
            if need_close:
                fp.close()
        return None

    # ── References ───────────────────────────────────────

    def add_reference(
        self,
        id: str,
        text: str,
        audio: str | Path | BinaryIO,
    ) -> dict:
        """Register a reference voice."""
        if isinstance(audio, (str, Path)):
            with open(audio, "rb") as f:
                return self._post_add_reference(id, text, f)
        return self._post_add_reference(id, text, audio)

    def _post_add_reference(
        self, id: str, text: str, audio_fp: BinaryIO
    ) -> dict:
        filename = getattr(audio_fp, "name", "audio.wav")
        if isinstance(filename, (str, Path)):
            filename = os.path.basename(filename)
        resp = self.session.post(
            f"{self.base_url}/v1/references/add",
            data={"id": id, "text": text},
            files={"audio": (filename, audio_fp)},
        )
        self._raise_for_error(resp)
        return resp.json()

    def list_references(self) -> dict:
        """List all registered reference voices."""
        resp = self.session.get(f"{self.base_url}/v1/references/list")
        self._raise_for_error(resp)
        return resp.json()

    def delete_reference(self, reference_id: str) -> dict:
        """Delete a reference voice."""
        resp = self.session.post(
            f"{self.base_url}/v1/references/delete",
            json={"reference_id": reference_id},
        )
        self._raise_for_error(resp)
        return resp.json()
