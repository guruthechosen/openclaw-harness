#!/bin/bash
cd /Volumes/formac/proj/safebot
exec ./target/release/openclaw-harness start --foreground 2>&1
