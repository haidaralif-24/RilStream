#include <Arduino.h>
#include <RilStream.h>

rst::Link link;

void setup() {
  link.begin(Serial3, 115200);  // Serial3 = the ESP8266 on the combined board
}

void loop() {
  link.update();  // non-blocking: never use delay() in loop()
}
