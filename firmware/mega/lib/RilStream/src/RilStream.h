#pragma once
#include <Arduino.h>

namespace rst {

class Link {
 public:
  void begin(HardwareSerial& port, unsigned long baud);
  void update();  // call once per loop(), see SPEC.md section 10

 private:
  HardwareSerial* port_ = nullptr;
};

}  // namespace rst
