# RilStream Protocol Specification

**Status:** Draft v0.1 (protocol version `1`)
This document defines the wire format and the behaviour of every component in the data path between a robot and a laptop. It is the contract between the C++ firmware library and the Rust host library. Where this document and the code disagree, fix one of them. Do not let them drift.

The key words MUST, SHOULD and MAY are used in the usual RFC 2119 sense.

---

## 1. Goals and non-goals

**Goals**

- Stream raw sensor data from a moving robot to a laptop in real time over Wi-Fi.
- Timestamp every sample at the moment it is captured on the robot.
- Make the robot **self-describing**: the host learns channels, fields, units and rates from the robot, with no host-side configuration.
- Survive packet loss, corruption and mid-stream connects without hanging or panicking.
- Keep the robot side small enough for an ATmega2560 (8 KB RAM).

**Non-goals (v0.1)**

- Reliable delivery. Lost packets are detected and counted, never retransmitted.
- Security or authentication. Assume a private hotspot.
- Laptop → robot commands (reserved for a later version).
- Absolute wall-clock time. Clock alignment is a host-side concern (see 9).

---

## 2. Architecture

```
Sensors ─> Mega (RilStream firmware lib) ─UART─> ESP8266 ═Wi-Fi/UDP═> Host (Rust) ─> consumers
           sample, timestamp,               bridge only            decode, stats,   dashboard,
           frame, COBS, CRC                                         buffer           logger, ...
```

| Component | Language | Responsibility |
|---|---|---|
| Firmware library | C++ (PlatformIO) | Sampling, timestamping, framing, descriptors |
| ESP8266 bridge | C++ (PlatformIO) | Forward frames to UDP, host discovery. Does not decode. |
| `rst-proto` | Rust, `no_std` | Frame format, COBS, CRC, descriptor encode/decode |
| `rst-host` | Rust, `std` | Transports, discovery, schema-driven decoding, stats, dashboard |

---

## 3. Conventions

- All multi-byte integers are **little-endian**.
- `f32` is IEEE 754 binary32, little-endian.
- Strings are `u8` length followed by that many ASCII bytes. No terminator. Length ≤ 15.
- Time is **microseconds** from the robot's `micros()` as `u32`.

---

## 4. Framing

Frames are delimited by COBS ([Consistent Overhead Byte Stuffing](https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing)) with `0x00` as the delimiter.

- The sender encodes a frame (section 5) with COBS and appends one `0x00`.
- The receiver accumulates bytes until `0x00`, then COBS-decodes the accumulated bytes.
- After any error (bad COBS, bad CRC, overflow) the receiver discards the bytes so far and resynchronises at the next `0x00`.
- **Maximum decoded frame size: 120 bytes.** With COBS overhead the encoded frame is at most 121 bytes. Buffers on the ESP8266 and host SHOULD be at least 128 bytes.
- Empty frames (two consecutive `0x00`) MUST be ignored.

---

## 5. Frame layout

Every frame, before COBS:

| Offset | Size | Field | Description |
|---|---|---|---|
| 0 | 1 | `version` | Protocol version. This spec is `1`. |
| 1 | 1 | `channel` | `0` = descriptor. `1..=255` = data channel. |
| 2 | 1 | `schema_id` | Identifies the robot's current configuration (section 7). |
| 3 | 2 | `seq` | Per-channel counter, wraps at 65535. Descriptors use their own counter on channel 0. |
| 5 | 4 | `t_us` | Robot time at capture, microseconds. Wraps about every 71.6 minutes. |
| 9 | n | `payload` | Data fields (section 6) or descriptor body (section 7). |
| 9+n | 2 | `crc16` | CRC of bytes `0 .. 9+n-1`. |

Minimum frame length is 11 bytes (empty payload).

### CRC

CRC-16/CCITT-FALSE: polynomial `0x1021`, init `0xFFFF`, no input or output reflection, xorout `0x0000`. Check value: `crc16("123456789") = 0x29B1`. A frame with a wrong CRC MUST be dropped and counted.

### Version handling

A receiver MUST drop frames whose `version` it does not support and count them separately from CRC errors.

---

## 6. Data channels

A data frame's `payload` is the concatenation of the channel's fields, in the order declared by its descriptor, with no padding. The payload length MUST equal the sum of the declared field sizes. A frame that does not match MUST be dropped and counted as a schema error.

### Field types

| Code | Type | Size |
|---|---|---|
| `0x01` | `u8` | 1 |
| `0x02` | `i8` | 1 |
| `0x03` | `u16` | 2 |
| `0x04` | `i16` | 2 |
| `0x05` | `u32` | 4 |
| `0x06` | `i32` | 4 |
| `0x07` | `f32` | 4 |

The physical value of a field is `raw × scale`. For `f32` fields the scale is normally `1.0`.

### Sampling rules

- `t_us` is the time the sample was captured, not the time it was sent.
- Cumulative quantities (for example encoder ticks) SHOULD be sent as running totals, not deltas, so a lost packet does not corrupt the integral.
- Channels are independent. Each has its own `seq`, rate and schema.

---

## 7. Descriptors (self-description)

A descriptor frame has `channel = 0`. Its payload describes **one** data channel:

| Size | Field | Description |
|---|---|---|
| 1 | `channel_id` | The data channel being described (`1..=255`) |
| 2 | `rate_hz` | Nominal sample rate, Hz (`0` = irregular) |
| 1+len | `name` | Channel name, for example `imu` |
| 1 | `n_fields` | Number of fields (1..=16) |
| per field | `type` (1), `scale` (`f32`, 4), `name` (1+len), `unit` (1+len) | Field definition |

The descriptor payload MUST fit in 109 bytes (120 − 9 header − 2 CRC). Keep names short.

### Schema id

`schema_id` is the **low byte of the CRC-16 of the concatenated descriptor payloads, ordered by `channel_id`**. It changes whenever the configuration changes. Every frame (descriptor or data) carries it.

### Transmission

- On boot the robot MUST send every descriptor once.
- The robot MUST then re-send descriptors continuously, one every 2 seconds, round-robin, so a host that connects late learns the schema within a few seconds.
- Descriptors count toward the bandwidth budget but are small (about 100 bytes every 2 s per channel).

### Host behaviour

- The host keys descriptors by `(schema_id, channel_id)`.
- A data frame is decodable only if the host holds the descriptor for its `(schema_id, channel)`. Otherwise the frame is counted as `unknown_schema` and dropped.
- On receiving a descriptor with a new `schema_id`, the host MUST discard all descriptors of the previous schema and reset per-channel sequence tracking. The robot has rebooted or been reconfigured.
- The host MUST NOT require any channel to be configured in advance. Anything it shows comes from descriptors.

---

## 8. Transport

### 8.1 Mega ↔ ESP8266 (UART)

- Mega `Serial3` to ESP8266, 115200 baud 8N1 by default (configurable, both ends must match).
- The byte stream is the COBS-framed stream from section 4.
- DIP switch settings are board-specific. See the board's documentation.

### 8.2 ESP8266 ↔ host (UDP)

| Port | Purpose |
|---|---|
| `9000` | Data: ESP → host |
| `9001` | Control: discovery and keepalive |

- **One UDP datagram carries exactly one COBS-encoded frame.** The trailing `0x00` MAY be omitted by the sender. A receiver MUST accept the frame with or without it.
- The ESP8266 does not decode frames. It buffers bytes until `0x00`, then sends the datagram. A frame that overflows the 128-byte buffer is dropped whole and the ESP resynchronises at the next `0x00`.
- The ESP8266 MUST keep draining the UART when Wi-Fi is down or no host is known, discarding frames, so the Mega is never blocked.
- The ESP SHOULD disable Wi-Fi modem sleep to reduce latency.

### 8.3 Discovery and keepalive (control port)

Control messages are plain ASCII, with no framing.

| Message | Direction | Meaning |
|---|---|---|
| `LINK?` | ESP → broadcast, port 9001 | "Is there a host?" Sent once per second while no host is known. |
| `LINK!` | Host → ESP, port 9001 | Reply to `LINK?`, and keepalive. |

Procedure:

1. The ESP joins Wi-Fi. While it has no host, it broadcasts `LINK?` every 1 s.
2. The host listens on port 9001. On `LINK?` it replies `LINK!` to the sender's address.
3. On `LINK!`, the ESP records the sender's IP as the host and starts sending frames to `host:9000`.
4. The host sends `LINK!` to the ESP every 1 s as keepalive.
5. If the ESP receives no `LINK!` for 5 s, it forgets the host and returns to step 1.

A static host address MAY be configured on the ESP instead, in which case steps 1 to 5 are skipped. Broadcast can be dropped by some access points. Static configuration is the fallback.

### 8.4 Other transports

The host library abstracts the byte source behind a `Transport` trait. USB serial (direct to the Mega, for development) and an in-memory mock (for tests) MUST behave identically to UDP from the decoder's point of view.

---

## 9. Host behaviour

### 9.1 Receive pipeline

```
bytes/datagram -> deframe (COBS) -> length + CRC check -> version check
   -> descriptor? -> update schema registry
   -> data?       -> look up schema -> decode fields -> Sample -> bounded buffer
```

Every stage that drops a frame increments a distinct counter.

### 9.2 Sequence and loss accounting

For each channel, with `prev` the last accepted `seq`:

- `gap = (seq - prev - 1) mod 65536`.
- If `gap` is small (less than 32768), add `gap` to `lost`.
- Otherwise treat the frame as reordered or duplicate and count it in `reordered`. Do not add to `lost`.

### 9.3 Timestamp unwrapping

`t_us` is a wrapping `u32`. The host extends it to 64 bits by treating a decrease of more than 2³¹ as a wrap. A decrease that is not a wrap (the robot rebooted) is detected by a `schema_id` change or a large backward jump and resets the extension.

### 9.4 Clock alignment (informative)

To compare robot time with host time (for example against camera frames), the host MAY estimate the offset as the minimum over a sliding window of `arrival_time - t_us_unwrapped`. The fastest packets carry the least network delay.

### 9.5 Buffering and back-pressure

- Consumers receive samples through a **bounded** buffer.
- When the buffer is full, the **oldest** sample is dropped. For monitoring, fresh data beats complete data.
- Slow consumers MUST NOT grow memory without bound or block the receiver.

### 9.6 Statistics

Per channel and overall:

| Stat | Meaning |
|---|---|
| `frames_ok` | Frames decoded successfully |
| `crc_errors` | Dropped for bad CRC |
| `cobs_errors` | Dropped for invalid COBS |
| `version_errors` | Unsupported version |
| `unknown_schema` | No descriptor for `(schema_id, channel)` |
| `schema_errors` | Payload length mismatch |
| `lost` | Estimated lost frames from `seq` gaps |
| `reordered` | Out-of-order or duplicate frames |
| `rate_hz` | Measured arrival rate |
| `jitter` | Spread of inter-arrival time versus nominal |

### 9.7 Robustness requirement

The decoder MUST NOT panic or allocate unboundedly on arbitrary input. This is verified by fuzzing and property tests (section 12).

---

## 10. Firmware library behaviour (informative)

The firmware API is not part of the wire contract, but the behaviour it must have is:

- Configuration declares sensors, pins, rates and channel layouts. Descriptors are generated from it, stored in flash, and never hand-written by the user.
- A single `update()` call in `loop()` services every channel. It is **non-blocking** and time-driven. The user's `loop()` MUST NOT call `delay()`.
- Timestamps are taken at capture time.
- The library owns its interrupts (for example encoder pins).
- Ultrasonic sensors are triggered round-robin, at least 60 ms per sensor, and read without blocking.

Sketch of the intended usage:

```cpp
#include <RilStream.h>

rst::Link link;

void setup() {
  link.begin(Serial3, 115200);
  link.addImu(100);                       // Hz, MPU6050 over I2C
  link.addEncoders(2, 3, 18, 19, 50);     // left/right pins, Hz
  link.addUltrasonic(15, {22, 23}, {24, 25}, {26, 27});
}

void loop() {
  link.update();
}
```

---

## 11. Dashboard interface (draft)

The host library MAY include a web dashboard behind a feature flag.

- HTTP server on `127.0.0.1:8080` serving one static page.
- WebSocket at `/ws`, JSON messages:
  - `{"type":"schema", ...}`: the current descriptors, sent on connect and on change.
  - `{"type":"samples", "channel":N, "t_us":[...], "values":[[...], ...]}`: batched at about 20 to 30 Hz.
  - `{"type":"stats", ...}`: link health, at about 2 Hz.
- The page builds one plot per field from the descriptor (name, unit, scale). It MUST NOT contain any hardcoded channel list.

---

## 12. Conformance and test vectors

Both implementations (C++ and Rust) MUST produce and accept these byte sequences exactly. Bytes are hexadecimal. These vectors are the ground truth for `test-vectors/`.

### 12.1 CRC check

`crc16("123456789")` = `0x29B1`

### 12.2 COBS

| Decoded | Encoded (without delimiter) |
|---|---|
| `00` | `01 01` |
| `11 22 00 33` | `03 11 22 02 33` |

### 12.3 Schema

One channel: `channel_id = 2`, 50 Hz, name `enc`, two `i32` fields (`l`, `r`), scale `1.0`, unit `tick`.

Descriptor payload (32 bytes):

```
02 32 00 03 65 6E 63 02 06 00 00 80 3F 01 6C 04 74 69 63 6B 06 00 00 80 3F 01 72 04 74 69 63 6B
```

CRC-16 of the payload = `0xDCB6`, so **`schema_id = 0xB6`**.

### 12.4 Descriptor frame

`version = 1`, `channel = 0`, `schema_id = 0xB6`, `seq = 0`, `t_us = 1000`.

Decoded (43 bytes):

```
01 00 B6 00 00 E8 03 00 00 02 32 00 03 65 6E 63 02 06 00 00 80 3F 01 6C 04 74 69 63 6B 06 00 00 80 3F 01 72 04 74 69 63 6B 73 7A
```

On the wire (COBS + delimiter, 45 bytes):

```
02 01 02 B6 01 03 E8 03 01 03 02 32 07 03 65 6E 63 02 06 01 0B 80 3F 01 6C 04 74 69 63 6B 06 01 0C 80 3F 01 72 04 74 69 63 6B 73 7A 00
```

### 12.5 Data frame

Channel 2, `schema_id = 0xB6`, `seq = 1`, `t_us = 1000000`, `l = 100`, `r = -100`.

Decoded (19 bytes):

```
01 02 B6 01 00 40 42 0F 00 64 00 00 00 9C FF FF FF 62 0B
```

CRC = `0x0B62`.

On the wire (COBS + delimiter, 21 bytes):

```
05 01 02 B6 01 04 40 42 0F 02 64 01 01 07 9C FF FF FF 62 0B 00
```

Expected decode with the descriptor from 12.4: channel `enc`, fields `l = 100 tick`, `r = -100 tick`, at `t_us = 1000000`.

### 12.6 Required tests

- Round-trip: encode then decode for every field type and for empty and maximum-size payloads.
- All vectors in this section, in both languages.
- **Fuzz and property tests:** random bytes, truncated frames, bit-flipped frames, and garbage between valid frames. The decoder never panics and recovers at the next valid frame.
- **Resync:** inject garbage mid-stream and show frames after it decode correctly.
- **Loss accounting:** drop, duplicate and reorder frames and verify `lost` and `reordered`.
- **Schema change:** switch `schema_id` mid-stream and verify old descriptors are discarded.
- **Timestamp wrap:** feed `t_us` across the `u32` wrap and verify monotonic 64-bit output.

---

## 13. Bandwidth budget (informative)

| Channel | Frame (on the wire) | Rate | Throughput |
|---|---|---|---|
| IMU (6 × `i16`) | about 25 B | 100 Hz | 2.5 KB/s |
| Encoders (2 × `i32`) | about 21 B | 50 Hz | 1.1 KB/s |
| Ultrasonic (3 × `u16`) | about 19 B | 15 Hz | 0.3 KB/s |
| Descriptors | about 100 B | 0.5 Hz per channel | 0.15 KB/s |
| **Total** | | | **about 4 KB/s** |

UART at 115200 baud carries about 11.5 KB/s, so this is roughly 35% utilisation. Raise the baud rate before raising sample rates.

---

## 14. Open questions

- Whether to add a laptop → robot command channel (motor control) in v0.2.
- Whether to add a `FLAGS` byte to the header (for example, "sample is derived" or "sensor error").
- Maximum number of channels per robot in practice (limited by descriptor round-robin latency).

---

## 15. Changelog

- **0.1** Initial draft.
