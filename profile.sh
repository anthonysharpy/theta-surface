#!/bin/bash

perf record -F 8000 --call-graph dwarf -- ./target/debug/ThetaSurface build-surface
hotspot perf.data