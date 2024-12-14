#!/bin/bash

. "$HOME/.cargo/env"

cd /home/oxideops/skin-trader || exit

git fetch

LOCAL_HASH=$(git rev-parse @)
REMOTE_HASH=$(git rev-parse '@{u}')

if [ "$LOCAL_HASH" != "$REMOTE_HASH" ]; then
	bitskins_hash=$(md5sum target/release/bitskins)
	dmarket_hash=$(md5sum target/release/dmarket)

	git reset --hard "$REMOTE_HASH"
	sqlx migrate run

	cargo build -p bitskins -r
	cargo build -p dmarket -r

	if [ "$bitskins_hash" != "$(md5sum target/release/bitskins)" ]; then
		sudo systemctl restart bitskins
	fi

	if [ "$dmarket_hash" != "$(md5sum target/release/dmarket)" ]; then
		sudo systemctl restart dmarket
	fi
fi
