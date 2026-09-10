#!/usr/bin/env python3
"""
Canglang (沧浪) 象棋音效程序化合成脚本 (CC0 / Public Domain / MIT 纯净版权证明)

本脚本使用纯数学阻尼正弦谐波方程程序化合成 4 个 44.1kHz 16位标准 PCM WAV 棋类音效：
1. move.wav:    80ms 清脆木质落子敲击声 (600Hz 物理主频 + 1200Hz 二次谐波 + 强指数阻尼衰减)
2. capture.wav: 120ms 低频共振吃子撞击声 (220Hz 基频 + 880Hz 泛音共振)
3. check.wav:   250ms 将军金石清鸣 (1046.5Hz C6 纯音 + 2093Hz 八度泛音 + 慢衰减)
4. win.wav:     450ms 终局胜局大三和弦 (C5 523.25Hz + E5 659.25Hz + G5 783.99Hz 纯正大三和弦)

所有音频样本完全由 Python 标准库生成，不依赖任何第三方二进制素材或录音采样，
具备 100% 独立生成的公共领域版权属性。
"""

import math
import os
import struct
import wave

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SOUNDS_DIR = os.path.join(ROOT_DIR, "public", "sounds")
SAMPLE_RATE = 44100


def write_wav(file_path: str, samples: list[float], sample_rate: int = SAMPLE_RATE):
    with wave.open(file_path, "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sample_rate)
        packed = [
            struct.pack("<h", max(-32767, min(32767, int(s * 32760)))) for s in samples
        ]
        w.writeframes(b"".join(packed))


def generate_sounds():
    os.makedirs(SOUNDS_DIR, exist_ok=True)
    rate = SAMPLE_RATE

    # 1. move.wav: 80ms 清脆木质敲击声
    dur_move = 0.08
    n_move = int(rate * dur_move)
    move_samples = [
        (
            math.sin(2 * math.pi * 600 * (i / rate)) * 0.7
            + math.sin(2 * math.pi * 1200 * (i / rate)) * 0.3
        )
        * math.exp(-(i / rate) * 60)
        for i in range(n_move)
    ]
    write_wav(os.path.join(SOUNDS_DIR, "move.wav"), move_samples, rate)

    # 2. capture.wav: 120ms 低频共振撞击声
    dur_cap = 0.12
    n_cap = int(rate * dur_cap)
    cap_samples = [
        (
            math.sin(2 * math.pi * 220 * (i / rate)) * 0.6
            + math.sin(2 * math.pi * 880 * (i / rate)) * 0.4
        )
        * math.exp(-(i / rate) * 40)
        for i in range(n_cap)
    ]
    write_wav(os.path.join(SOUNDS_DIR, "capture.wav"), cap_samples, rate)

    # 3. check.wav: 250ms 将军金石清鸣
    dur_check = 0.25
    n_check = int(rate * dur_check)
    check_samples = [
        (
            math.sin(2 * math.pi * 1046.5 * (i / rate)) * 0.7
            + math.sin(2 * math.pi * 2093.0 * (i / rate)) * 0.3
        )
        * math.exp(-(i / rate) * 15)
        for i in range(n_check)
    ]
    write_wav(os.path.join(SOUNDS_DIR, "check.wav"), check_samples, rate)

    # 4. win.wav: 450ms 胜局大三和弦
    dur_win = 0.45
    n_win = int(rate * dur_win)
    win_samples = [
        (
            math.sin(2 * math.pi * 523.25 * (i / rate)) * 0.35
            + math.sin(2 * math.pi * 659.25 * (i / rate)) * 0.35
            + math.sin(2 * math.pi * 783.99 * (i / rate)) * 0.30
        )
        * math.exp(-(i / rate) * 8)
        for i in range(n_win)
    ]
    write_wav(os.path.join(SOUNDS_DIR, "win.wav"), win_samples, rate)

    print(f"✓ 已生成 4 个程序化纯净版权 WAV 音效文件 -> {SOUNDS_DIR}")


if __name__ == "__main__":
    generate_sounds()
