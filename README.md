# RilStream

*In code, RilStream is abbreviated as `rst`: crates `rst-proto` and `rst-host`, C++ namespace `rst`.*

Realtime raw sensor streaming from a moving robot to a laptop, over Wi-Fi, with a Rust host library and a live web dashboard.

The robot describes itself: it announces its sensors, fields, units and rates, and the host (including the dashboard) shows exactly what it is told. Nothing is configured on the laptop.

**Status:** design phase. The protocol is specified in [`SPEC.md`](SPEC.md). No code has been written yet, and every command below marked *(planned)* does not exist yet.

---

## Why

Mobile robots need to be watched while they move, so a cable is not an option. Existing tools either assume a heavy stack (ROS) or leave framing, loss handling and timestamping to you. RilStream is deliberately small:

- **One line on the robot.** Declare sensors once, call `update()` in `loop()`.
- **Honest timestamps.** Every sample is stamped on the robot at capture, so Wi-Fi jitter does not corrupt your data.
- **Loss is visible, not hidden.** Sequence numbers and CRCs turn dropped or corrupted packets into numbers you can see.
- **Fresh data wins.** Slow consumers drop the oldest samples. Memory never grows without bound.
- **Nothing to configure on the host.** The robot sends a descriptor of its channels, and the dashboard builds itself from it.

## How it works

```
Sensors ─> Mega (RilStream firmware) ─UART─> ESP8266 ═Wi-Fi/UDP═> Host (Rust) ─> web dashboard
           sample, timestamp,            bridge only            decode, stats,    logger, ...
           frame, COBS, CRC                                      buffer
```

1. The firmware samples each sensor at its configured rate and stamps `micros()` at capture.
2. Each sample becomes a small binary frame (header, raw fields, CRC-16), COBS-encoded with `0x00` as the delimiter.
3. The ESP8266 forwards each frame as one UDP datagram. It finds the laptop itself with a broadcast handshake, so no IP address is hardcoded.
4. The Rust host verifies the CRC, tracks sequence gaps, decodes fields using the robot's descriptors, and pushes samples into a bounded buffer.
5. Consumers (the dashboard, a CSV logger, your own code) read from that buffer.

The full wire format, discovery procedure, host behaviour and test vectors are in [`SPEC.md`](SPEC.md).

## Planned features

- Self-describing channels (descriptors sent at boot and repeated periodically)
- Per-channel loss, CRC error, reorder and jitter statistics
- Live web dashboard: plots built from descriptors, plus a link-health panel
- Transports: UDP over Wi-Fi, USB serial for development, in-memory mock for tests
- `no_std` protocol crate, shared by anything that wants to speak the format
- Golden test vectors checked by both the C++ and Rust implementations
- Fuzz and property tests: the decoder never panics on arbitrary input

## Hardware (reference setup)

| Part | Role |
|---|---|
| Arduino Mega 2560 + ESP8266 (combined board) | Sampling and Wi-Fi |
| MPU6050 | IMU over I2C |
| 3 × HC-SR04 | Ultrasonic distance |
| 2 × JGA12-N20 motors with encoders | Motion and odometry |
| L298N | Motor driver |
| 2S 18650 pack | Power |

A camera with ArUco tags is used separately as ground truth. Aligning robot and camera time is described in `SPEC.md`, section 9.4.

The library is not tied to this hardware. Any source that emits the frames in `SPEC.md` works.

## Repository layout (planned)

```
RilStream/
├── SPEC.md
├── README.md
├── rst-proto/         # no_std Rust: frames, COBS, CRC, descriptors
├── rst-host/          # std Rust: transports, discovery, schemas, stats, dashboard
│   └── examples/      # fake_robot, print, dashboard
├── firmware/
│   ├── mega/          # PlatformIO: RilStream firmware library + example sketch
│   └── esp/           # PlatformIO: UART to UDP bridge
└── test-vectors/      # shared golden frames (from SPEC.md section 12)
```

## Usage sketches

These show the intended API. They are not working code yet.

**Robot (Mega):**

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
  link.update();   // non-blocking, so never use delay() in loop()
  // your control code here
}
```

**Laptop (Rust):**

```rust
let mut link = rst_host::Link::open_udp()?;   // discovers the robot itself

for sample in link.samples() {
    println!("{} {:?}", sample.channel, sample.values);
}
```

**Dashboard:** *(planned)*

```bash
cargo run -p rst-host --example dashboard
# then open http://localhost:8080
```

## Development

The workflow is CLI-first, with Zed as the editor.

```bash
# firmware (PlatformIO)
pio run -d firmware/mega -t upload
pio run -d firmware/esp  -t upload
pio device monitor -d firmware/mega -b 115200
pio run -d firmware/mega -t compiledb     # compile_commands.json for clangd

# host (Rust)
cargo test
cargo run -p rst-host --example dashboard
```

**Network notes**

- The ESP8266 is 2.4 GHz only and supports WPA2-PSK, not WPA-Enterprise.
- Use a phone hotspot (with 2.4 GHz or "Maximize Compatibility" enabled) or a dedicated router. School or campus Wi-Fi often isolates clients and will silently break discovery.
- On macOS, allow incoming connections for the host binary when prompted, or UDP packets are blocked.
- Wi-Fi credentials live in `firmware/esp/include/config.h`, which is gitignored. Copy `config.example.h` and fill it in.

## Testing

Every layer is meant to be testable without hardware:

- Round-trip and vector tests for the frame format, in Rust and C++
- A mock transport that feeds bytes from memory
- A fake robot (a Rust binary on the laptop) that sends real frames over UDP
- Fuzz tests for the decoder, and resync tests that inject garbage mid-stream

## Roadmap

- [ ] `rst-proto`: header, field types, descriptors, COBS, CRC, with test vectors
- [ ] `rst-host`: mock transport, deframer, schema registry, stats
- [ ] Fake robot over UDP, and the discovery handshake
- [ ] Firmware encoder in C++, verified against the shared vectors over USB serial
- [ ] ESP8266 bridge over Wi-Fi
- [ ] Firmware drivers: MPU6050, encoders, HC-SR04
- [ ] Web dashboard
- [ ] CSV logger, then clock alignment with the camera
- [ ] Fuzzing and throughput measurements in this README
- [ ] Publish (crates.io for the Rust crates, PlatformIO registry for the firmware library)

## License

To be decided.
