# Registry Examples Makefile
#
# Build C++ registry examples

CXX = g++
CXXFLAGS = -std=c++20 -O2 -I../../cpp/include
LDFLAGS = -L../../cpp -lactors -lpthread -lzmq

TARGETS = registry_pong registry_ping

all: $(TARGETS)

registry_pong: registry_pong.cpp
	$(CXX) $(CXXFLAGS) $< -o $@ $(LDFLAGS)

registry_ping: registry_ping.cpp
	$(CXX) $(CXXFLAGS) $< -o $@ $(LDFLAGS)

clean:
	rm -f $(TARGETS)

.PHONY: all clean
