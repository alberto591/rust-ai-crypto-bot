#!/bin/bash
export PROTOC=/usr/bin/protoc
export PATH=$PATH:/usr/bin
cd /opt/mev-bot-src
source $HOME/.cargo/env
nohup cargo build --release -p engine > /tmp/build.log 2>&1 &
echo "🚀 Build ignited in background."
