#!/bin/sh
sed "s/^/\/\/\!/g" README.md > lib.rs
sed "/^\/\/\!/d" src/lib.rs >> lib.rs
mv -f lib.rs src/

