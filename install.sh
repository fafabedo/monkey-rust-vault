#!/bin/bash
set -e

export PATH="$HOME/.cargo/bin:$PATH"

cargo build --release
