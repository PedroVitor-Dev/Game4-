# Transfer adapter boundary

The implemented experimental desktop adapter lives in [`../torrents/`](../torrents/). It embeds librqbit 9.1.0 behind explicit native commands and custom read-only source/new-file destination storage. Transfer state remains separate from catalog content identity. See [desktop transfers](../../../../../docs/bittorrent.md) for supported public-v1 metainfo, connectivity settings, limits and runtime-only lifecycle.

The earlier [isolated probe](../../../../../tools/torrent-probe/README.md) remains a separate evaluation tool. Its two-PC Tailscale evidence does not close production selection or native-network acceptance for the desktop integration; see [the evaluation](../../../../../docs/torrent-engine-evaluation.md).
