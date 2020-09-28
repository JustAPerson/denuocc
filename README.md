# Denuo C Compiler

> denuo [adverb]:
> new, over again, from a fresh beginning

**denuocc** is yet another case of [Not Invented Here Syndrome][NIH]. Compilers
are an interesting subject in computer science--one I would like to explore on
my own.

[NIH]: https://en.wikipedia.org/wiki/Not_invented_here

# Current State

I've written [a lot of code][alot], but I don't have anywhere near a
functioning compiler yet. I've been revising a lot of core functionality and
building out infrastructure in order to avoid making anything that's really
difficult to replace later.  One of the core features I've put a lot of care
into is location tracking (recording where in the source code some feature of
the program is derived from). On top of that, I've implemented the majority of
the [C preprocessor][cpp]. These two features intersect in a nontrivial way: a
location in C source code can have two different paths from which it derives.
It can be expanded as a result of file inclusion (`#include <stdio.h>`) or
macro expansion (`#define FOO`). This can make it really difficult to provide
meaningful error messages, but I think I've finally gotten the right
representation for this, and so now I've started working on a custom parser.
With a lot of massaging, C can be parsed as an [`LL(2)`][ll2] language, but
there are very few resources describing how to properly parse `LL(k)` languages
where `k > 1`, so after scouring some ancient text books, I've managed to
implement [this utility][gt] that simplifies the complicated task of writing
any `LL(k)` parser.

[alot]: https://github.com/JustAPerson/denuocc/graphs/contributors
[ll2]: https://en.wikipedia.org/wiki/LL_parser
[gt]: https://github.com/JustAPerson/denuocc/tree/master/tools/grammar_tool
[cpp]: https://github.com/JustAPerson/denuocc/blob/96e3a408de7af7cbdc0ba619596883109a6a3ea8/src/front/c/preprocessor.rs#L1716-L1787

# Running

You can invoke the compiler on a specific file using `cargo run -- <filename>`
like so
```
$ cat /tmp/broken.c
#define NOT_A_LEGAL_INCLUDE_PATH 3+2
#include NOT_A_LEGAL_INCLUDE_PATH

$ cargo run -- /tmp/broken.c
fatal error: expected `<FILENAME>`, `"FILENAME"`, or a macro that expands to either of those
  /tmp/broken.c:2:10
2 | #include NOT_A_LEGAL_INCLUDE_PATH
```

To get an idea of what's going on, try running the code with verbose logging
using the `RUST_LOG` environment variable

```
$ RUST_LOG="denuocc=trace" cargo run -- /tmp/broken.c
[2020-09-28T17:20:53Z DEBUG denuocc::driver] Driver::process_clap_matches() matches = ArgMatches { args: {"FILES": MatchedArg { occurs: 1, indices: [1], vals: ["/tmp/broken.c"] }}, subcommand: None, usage: Some("USAGE:\n    denuocc [OPTIONS] <FILES>...") }
[2020-09-28T17:20:53Z INFO  denuocc::core::flags] Flags::process_clap_matches() passes: [StateReadInput, Phase1, Phase2, Phase3, Phase4, Phase5, Phase6]
[2020-09-28T17:20:53Z INFO  denuocc::driver] Driver::add_input_file() path = "/tmp/broken.c"
[2020-09-28T17:20:53Z INFO  denuocc::driver] Driver::add_input_file() reading from file
[2020-09-28T17:20:53Z INFO  denuocc::driver] Driver::run_all() all names = ["/tmp/broken.c"]
[2020-09-28T17:20:53Z INFO  denuocc::driver] Driver::run_all() running name = "/tmp/broken.c"
[2020-09-28T17:20:53Z DEBUG denuocc::front::c::tuctx] TUCtx::run() tu alias "/tmp/broken.c" running pass StateReadInput
[2020-09-28T17:20:53Z DEBUG denuocc::front::c::tuctx] TUCtx::run() fatal false
[2020-09-28T17:20:53Z DEBUG denuocc::front::c::tuctx] TUCtx::run() tu alias "/tmp/broken.c" running pass Phase1
[2020-09-28T17:20:53Z TRACE denuocc::front::c::minor] convert_trigraphs() output[0] = CharToken { value: '#', span: TextSpan { pos: TextPosition { input: 0, absolute: 0 }, len: 1 } }
[2020-09-28T17:20:53Z TRACE denuocc::front::c::minor] convert_trigraphs() output[1] = CharToken { value: 'd', span: TextSpan { pos: TextPosition { input: 0, absolute: 1 }, len: 1 } }
...
```

# License
This project is dual licensed under the terms of the MIT license and the Apache
License Version 2.0 at your option. See [./LICENSE-MIT][MIT] and
[./LICENSE-APACHE][APACHE] for details.

[MIT]: ./LICENSE-MIT
[APACHE]: ./LICENSE-APACHE


## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
