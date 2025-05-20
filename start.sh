#!/bin/sh
set -e

# Start Alloy in the background
alloy run /etc/alloy/alloy.river &

# Start your Actix Web app
exec slipway_registry
