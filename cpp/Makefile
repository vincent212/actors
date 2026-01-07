LIBSRC = Actor.cpp Manager.cpp RegistryClient.cpp RustActorRefStub.cpp
NAM = actors

CXX = g++
CXXFLAGS = -std=c++20 -O2 -Iinclude
LDFLAGS = -lpthread

# Remote actor support (ZMQ + JSON)
REMOTE_LDFLAGS = -lzmq

OBJS = $(LIBSRC:.cpp=.o)
LIB = lib$(NAM).a

all: $(LIB)

$(LIB): $(OBJS)
	ar rcs $@ $^

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@

# Example targets (registry examples are in ~/actors/registry/examples/)
examples: examples/ping_pong examples/remote_pong examples/remote_ping

examples/ping_pong: examples/ping_pong.cpp $(LIB)
	$(CXX) $(CXXFLAGS) $< -o $@ -L. -l$(NAM) $(LDFLAGS)

examples/remote_pong: examples/remote_pong.cpp $(LIB)
	$(CXX) $(CXXFLAGS) $< -o $@ -L. -l$(NAM) $(LDFLAGS) $(REMOTE_LDFLAGS)

examples/remote_ping: examples/remote_ping.cpp $(LIB)
	$(CXX) $(CXXFLAGS) $< -o $@ -L. -l$(NAM) $(LDFLAGS) $(REMOTE_LDFLAGS)

# Test targets (requires Google Test)
TEST_SRC = $(wildcard tests/test_*.cpp)
TEST_BIN = run_tests

test: $(TEST_BIN)
	./$(TEST_BIN)

$(TEST_BIN): $(TEST_SRC) $(LIB)
	$(CXX) $(CXXFLAGS) $(TEST_SRC) -o $@ -L. -l$(NAM) $(LDFLAGS) -lgtest -lgtest_main

clean:
	rm -f $(OBJS) $(LIB) $(TEST_BIN) examples/ping_pong examples/remote_pong examples/remote_ping

.PHONY: all clean examples test
