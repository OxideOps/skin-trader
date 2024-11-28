#!/bin/bash

. "$HOME/.cargo/env"

cd /home/oxideops/skin-trader

git fetch

LOCAL_HASH=$(git rev-parse @)
REMOTE_HASH=$(git rev-parse @{u})

if [ "$LOCAL_HASH" != "$REMOTE_HASH" ]; then
    git reset --hard "$REMOTE_HASH"
    sqlx migrate run
    cargo build -p bot -r
    sudo systemctl restart skin-trader
fi

