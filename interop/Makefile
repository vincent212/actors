# actors-interop Makefile
#
# Builds the C++/Rust FFI interop layer for actors

CXX = g++
CXXFLAGS = -std=c++17 -O2 -Wall -Wextra -fPIC
INCLUDES = -I$(HOME)/actors/include -I. -Igenerated/cpp -Imessages

# Paths
ACTORS_CPP = $(HOME)/actors
ACTORS_RUST = $(HOME)/actors/rust
GENERATED_CPP = generated/cpp
GENERATED_RUST = generated/rust

# Targets
.PHONY: all generate cpp rust clean

all: generate cpp rust

# Generate code from message definitions
generate:
	@echo "=== Generating C++ and Rust code from messages/interop_messages.h ==="
	python3 codegen/generate.py messages/interop_messages.h generated
	@echo ""

# Build C++ bridge object file (for linking into examples)
cpp: $(GENERATED_CPP)/CppActorBridge.cpp lib
	@echo "=== Building C++ bridge object file ==="
	$(CXX) $(CXXFLAGS) $(INCLUDES) -c -o lib/CppActorBridge.o \
		$(GENERATED_CPP)/CppActorBridge.cpp
	@echo "Built: lib/CppActorBridge.o"
	@echo ""

# Build Rust library (uses Cargo)
rust:
	@echo "=== Building Rust interop library ==="
	cd rust && cargo build --release
	@echo "Built: rust/target/release/libactors_interop.so"
	@echo ""

# Create lib directory
lib:
	mkdir -p lib

# Install: copy headers and libraries to standard locations
install: all
	@echo "=== Installing headers and libraries ==="
	mkdir -p $(HOME)/actors-interop/include/interop
	cp $(GENERATED_CPP)/InteropMessages.hpp $(HOME)/actors-interop/include/interop/
	cp $(GENERATED_CPP)/RustActorIF.hpp $(HOME)/actors-interop/include/interop/
	cp $(GENERATED_CPP)/CppActorBridge.hpp $(HOME)/actors-interop/include/interop/
	cp messages/interop_messages.h $(HOME)/actors-interop/include/interop/
	@echo "Headers installed to $(HOME)/actors-interop/include/"
	@echo ""

clean:
	rm -rf lib/*.so
	rm -rf rust/target
	rm -rf generated/cpp/*.hpp generated/cpp/*.cpp
	rm -rf generated/rust/*.rs

# Development helpers
.PHONY: format check

format:
	clang-format -i $(GENERATED_CPP)/*.hpp $(GENERATED_CPP)/*.cpp
	cd rust && cargo fmt

check:
	@echo "=== Checking C++ compilation ==="
	$(CXX) $(CXXFLAGS) $(INCLUDES) -c -o /dev/null $(GENERATED_CPP)/CppActorBridge.cpp 2>&1 || true
	@echo ""
	@echo "=== Checking Rust compilation ==="
	cd rust && cargo check 2>&1 || true
