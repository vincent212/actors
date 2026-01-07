/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski, & M2 Tech

Stub implementation of RustActorRef::send() for non-interop builds.
If you're using C++/Rust interop, link with the interop library instead.

*/

#include "actors/ActorRef.hpp"
#include <stdexcept>

namespace actors {

void RustActorRef::send(const Message* m, Actor*) {
    delete m;
    throw std::runtime_error("RustActorRef::send() not available - link with interop library for C++/Rust communication");
}

} // namespace actors
