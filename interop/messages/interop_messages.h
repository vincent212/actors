/*****************************************************************

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2024 Vincent Maciejewski, Quant Enterprises & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

Permission is hereby granted, free of charge, to any person
obtaining a copy of this software and associated documentation
files (the "Software"), to deal in the Software without
restriction, including without limitation the rights to use,
copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the
Software is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES
OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.

https://opensource.org/licenses/MIT

*****************************************************************/

#ifndef INTEROP_MESSAGES_H
#define INTEROP_MESSAGES_H

#include <stdint.h>

/*
 * Interop message definitions for C++/Rust FFI
 *
 * These structs use C-compatible types and layout.
 * Both C++ and Rust code generators will produce matching
 * struct definitions from this file.
 *
 * Rules:
 * - Use fixed-width integers (int32_t, int64_t, not int/long)
 * - Use int32_t for booleans (1=true, 0=false)
 * - Use interop_string for strings (fixed-size, no heap)
 * - Message IDs start at 1000 to avoid conflicts with internal messages
 */

/* Annotation macro for code generator - parsed but not compiled */
#define INTEROP_MESSAGE(name, id)

/* Fixed-size string for FFI (no heap allocation) */
#define INTEROP_STRING_MAX 64

typedef struct {
    char data[INTEROP_STRING_MAX];
    uint32_t len;
} interop_string;

/* ============================================================
 * Message Definitions
 * ============================================================ */

INTEROP_MESSAGE(Ping, 1000)
typedef struct {
    int32_t count;
} Ping;

INTEROP_MESSAGE(Pong, 1001)
typedef struct {
    int32_t count;
} Pong;

INTEROP_MESSAGE(DataRequest, 1002)
typedef struct {
    int32_t request_id;
    interop_string symbol;
} DataRequest;

INTEROP_MESSAGE(DataResponse, 1003)
typedef struct {
    int32_t request_id;
    double value;
    int32_t found;  /* bool: 1=true, 0=false */
} DataResponse;

/* ============================================================
 * Pub/Sub Messages for cross-language subscription patterns
 * ============================================================ */

INTEROP_MESSAGE(Subscribe, 1010)
typedef struct {
    char topic[32];
} Subscribe;

INTEROP_MESSAGE(Unsubscribe, 1011)
typedef struct {
    char topic[32];
} Unsubscribe;

INTEROP_MESSAGE(MarketUpdate, 1012)
typedef struct {
    char symbol[8];
    double price;
    int64_t timestamp;
    int32_t volume;
} MarketUpdate;

/* ============================================================
 * Example: Market Data with arrays
 * ============================================================ */

INTEROP_MESSAGE(MarketDepth, 1013)
typedef struct {
    char symbol[8];
    int32_t num_levels;
    double bid_prices[5];
    double ask_prices[5];
    int32_t bid_sizes[5];
    int32_t ask_sizes[5];
} MarketDepth;

#endif /* INTEROP_MESSAGES_H */
