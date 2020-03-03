#!/bin/bash
THISDIR=$(dirname $(realpath $0))
pushd $THISDIR/../../../tools/grammar_tool/
cargo run -- first -k2 $THISDIR/C.yacc > $THISDIR/C.first
cargo run -- follow -k2 $THISDIR/C.yacc > $THISDIR/C.follow
popd
