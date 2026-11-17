#include "RilStream.h"

namespace rst {

void Link::begin(HardwareSerial& port, unsigned long baud) {
  port_ = &port;
  port_->begin(baud);
  // TODO: register channels, build descriptors (SPEC.md section 7)
}

void Link::update() {
  // TODO: sample due channels, frame, COBS, CRC, write to port_
}

}  // namespace rst
