# Pub/Sub Example Makefile

CXX = g++
CXXFLAGS = -std=c++17 -O2 -Wall -Wextra -g

INCLUDES = \
    -I$(HOME)/actors-cpp/include \
    -I$(HOME)/actors-interop/generated/cpp \
    -I$(HOME)/actors-interop/cpp/include/interop \
    -I$(HOME)/actors-interop/messages

ACTORS_CPP_LIB = $(HOME)/actors-cpp/src/libactors.a
RUST_LIB = $(HOME)/actors-interop/rust/target/release/libactors_interop.a

LDFLAGS = -lpthread -ldl

.PHONY: all clean rust

all: pubsub

rust:
	@echo "=== Building Rust library ==="
	cd $(HOME)/actors-interop/rust && cargo build --release

pubsub: main.cpp rust $(ACTORS_CPP_LIB)
	@echo "=== Building pubsub example ==="
	$(CXX) $(CXXFLAGS) $(INCLUDES) -o $@ \
		main.cpp \
		$(HOME)/actors-interop/generated/cpp/CppActorBridge.cpp \
		$(HOME)/actors-interop/cpp/src/RustActorRef.cpp \
		$(ACTORS_CPP_LIB) \
		$(RUST_LIB) \
		$(LDFLAGS)
	@echo "Built: pubsub"

clean:
	rm -f pubsub
