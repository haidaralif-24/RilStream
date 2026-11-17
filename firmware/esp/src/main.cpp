// UART -> UDP bridge for RilStream. Draft: see SPEC.md section 8.
#include <Arduino.h>
#include <ESP8266WiFi.h>
#include <WiFiUdp.h>

#include "config.h"

static WiFiUDP data;
static WiFiUDP ctrl;
static IPAddress host;
static bool haveHost = false;
static bool overflow = false;
static uint32_t lastHostMs = 0;
static uint32_t lastBeaconMs = 0;
static uint8_t buf[128];  // max encoded frame is 121 bytes
static size_t n = 0;

void setup() {
  Serial.begin(UART_BAUD);
  WiFi.persistent(false);
  WiFi.mode(WIFI_STA);
  WiFi.setSleepMode(WIFI_NONE_SLEEP);  // modem sleep adds latency
  WiFi.setAutoReconnect(true);
  WiFi.begin(WIFI_SSID, WIFI_PASS);
  ctrl.begin(CTRL_PORT);
}

void loop() {
  const bool up = WiFi.status() == WL_CONNECTED;

  // Control port: "LINK!" from the host is both the reply and the keepalive.
  if (ctrl.parsePacket() > 0) {
    char msg[8] = {0};
    ctrl.read(msg, sizeof(msg) - 1);
    if (strncmp(msg, "LINK!", 5) == 0) {
      host = ctrl.remoteIP();
      haveHost = true;
      lastHostMs = millis();
    }
  }
  if (haveHost && millis() - lastHostMs > 5000) haveHost = false;

  // No host yet: broadcast "LINK?" once per second.
  if (up && !haveHost && millis() - lastBeaconMs > 1000) {
    IPAddress bc(static_cast<uint32_t>(WiFi.localIP()) |
                 ~static_cast<uint32_t>(WiFi.subnetMask()));
    ctrl.beginPacket(bc, CTRL_PORT);
    ctrl.write("LINK?");
    ctrl.endPacket();
    lastBeaconMs = millis();
  }

  // Always drain the UART, even when offline, so the Mega is never blocked.
  while (Serial.available()) {
    const uint8_t b = Serial.read();
    if (b == 0x00) {
      if (!overflow && n > 0 && up && haveHost) {
        data.beginPacket(host, DATA_PORT);
        data.write(buf, n);  // one frame = one datagram
        data.endPacket();
      }
      n = 0;
      overflow = false;
    } else if (n < sizeof(buf)) {
      buf[n++] = b;
    } else {
      overflow = true;  // drop this frame, resync at the next 0x00
    }
  }
}
