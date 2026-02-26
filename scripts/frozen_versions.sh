#!/usr/bin/env bash

# Canonical frozen version policy for ArqonPilot.
# Source this file from guardrail scripts to avoid drift.

export PILOT_CORE_RUST_VERSION="1.82.0"
export PILOT_PACKAGING_RUST_VERSION="1.88.0"

# Protobuf 4.25.8 corresponds to protoc 25.8 tag/binaries.
export PILOT_PROTOBUF_VERSION="4.25.8"
export PILOT_PROTOC_VERSION="25.8"
