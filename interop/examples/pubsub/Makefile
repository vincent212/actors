# Pub/Sub Example Makefile

CXX = g++
CXXFLAGS = -std=c++20 -O2 -Wall -Wextra -g

# Include paths
INCLUDES = \
    -I$(HOME)/actors/cpp/include \
    -I$(HOME)/actors/interop/generated/cpp \
    -I$(HOME)/actors/interop/messages

# Libraries
ACTORS_CPP_LIB = $(HOME)/actors/cpp/libactors.a
RUST_LIB = $(HOME)/actors/interop/rust/target/release/libactors_interop.a

# Link flags
LDFLAGS = -lpthread -ldl

.PHONY: all clean rust

all: pubsub

# Build Rust library first
rust:
	@echo "=== Building Rust library ==="
	cd $(HOME)/actors/interop/rust && cargo build --release
	@echo ""

# Build the example
pubsub: main.cpp rust $(ACTORS_CPP_LIB) $(HOME)/actors/interop/generated/cpp/CppActorBridge.cpp
	@echo "=== Building pubsub example ==="
	$(CXX) $(CXXFLAGS) $(INCLUDES) -o $@ \
		main.cpp \
		$(HOME)/actors/interop/generated/cpp/CppActorBridge.cpp \
		$(HOME)/actors/interop/cpp/src/RustActorRef.cpp \
		$(ACTORS_CPP_LIB) \
		$(RUST_LIB) \
		$(LDFLAGS)
	@echo ""
	@echo "Built: pubsub"
	@echo "Run with: ./pubsub"

clean:
	rm -f pubsub
