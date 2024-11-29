#!/bin/bash

. "$HOME/.cargo/env"

cd /home/oxideops/skin-trader || exit

git fetch

LOCAL_HASH=$(git rev-parse @)
REMOTE_HASH=$(git rev-parse '@{u}')

if [ "$LOCAL_HASH" != "$REMOTE_HASH" ]; then
    git reset --hard "$REMOTE_HASH"
    sqlx migrate run
    cargo build -p bots -r
    sudo systemctl restart bitskins-bot
fi

