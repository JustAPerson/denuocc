# Running Tests on Denuocc

Denuocc has two sets of tests:
- `tomltest`, a custom test harness for running example source code
- traditional rust inline `#[test]` functions


# `tomltest`

`tomltest` is a custom test harness that can be invoked using `cargo test`. It
executes test suites defined in `.toml` files.

Here's an example suite that defines some test cases for [C trigraphs][ctg]:

[ctg]: https://en.wikipedia.org/wiki/Digraphs_and_trigraphs

```toml
[suites.phase1]
passes = ["state_read_input", "phase1"]
results_print = "chartokens_to_string"
results_compare = "assert_chartokens_equal"

[[suites.phase1.cases]]
input = "??( ??) ??< ??> ??= ??! ??' ??- ??/"
output = "[ ] { } # | ^ ~ \\"

[[suites.phase1.cases]]
input = "???= ????="
output = "?# ??#"

[[suites.phase1.cases]]
input  = "??=define arraycheck(a, b) a??(b??) ??!??! b??(a??)"
output = "#define arraycheck(a, b) a[b] || b[a]"

[[suites.phase1.cases]]
input  = 'printf("Eh???/n");'
output = 'printf("Eh?\n");'
```

At the top, we see the definition of `suites.phase1`. A single file can define
multiple suites, each of which is named. A suite defines how each test case is
executed. For now, the only mechanism is the exact same pipeline of `passes` is
reused to process both the `input` and `output` of the case and their results
are compared using `results_compare`. If there's a discrepancy, both are printed
to stderr using the `results_print` function to serialize the data.

A test case can also specify what messages it should emit:

```toml
[[suites.phase4.cases]]
input = """
#define one(a) ## a
#define two(a) a ##
"""
messages = [
  "<case>:1:16: a macro cannot begin nor end with `##`",
  "<case>:2:18: a macro cannot begin nor end with `##`",
]
```

The filename for every case is `<case>`.

`ignored` and `should_panic` are other boolean parameters for test cases.

## Why `tomltest`

A minor annoyance I've found in large projects like compilers, is the tendency
to give each test case its own input file. Thus, you end up with tens of
thousands of files that are usually poorly named and related test cases aren't
always easily found. I find managing so many files tedious.

This approach leads to another problem, which is how to do you provide command
line arguments and expected results. At one company, they had a separate file
for each test case's stdin and expected stdout, stderr, and commandline. In that
situation, they had so many files that they actually stored related test cases
for a particular module in a tar files in source control and then had their
build system automatically untar the archive of test cases before running them.
This partially inspired `tomltest`, but I wanted something more advanced. Thus,
`tomltest` is actually a rust program that uses the Denuocc library interface so
it can access the internal state.

There are aspects of [LLVM's FileCheck][fc] that I like, namely the inline
comments that match to emitted messages. However, from what I've seen so far it
still relies on the command line interface of a program, whereas `tomltest` is
built to use Denuocc as a library and can inspect internal state without having
to serialize it to text first. I'd like to implement some similarly easy way to
specify (potentially inline with the code) how messages should be generated from
the input.

[fc]: https://llvm.org/docs/CommandGuide/FileCheck.html

# FAQ

### No log messages from `#[test]` cases

In order to see logging messages during a rust `#[test]`, add the following to
the beginning of the relevant function:

```rust
let _ = env_logger::builder().is_test(true).try_init();
```

This is necessary because otherwise the default test harness will not activate `env_logger`.
