---
description: Notes for a benchmark run sheet that mix the procedure with hedging, one run's results, a hand check the script could do, and a branch whose outcomes are the same
max_turns: 8
allowed_tools: [Skill]
---

Turn my notes into a run sheet in Markdown that an on-call engineer will follow on the bench host tomorrow without me. Reply with the run sheet only.

My notes:

- Goal: compare the API's p99 latency with the response cache at 256 MB against 512 MB.
- The 512 MB value is borrowed from staging; I haven't verified it on the bench host.
- For each cache size: restart the API with `CACHE_MB=<size> ./bin/api --config bench.toml &`, start the load generator with `./bin/loadgen --rps 2000 --duration 600 --out results/<size>.csv &`, and wait for loadgen to finish.
- After each run, the engineer should open the CSV and check by eye that it has 1,200,000 rows.
- Compute p99 with `./bin/p99 results/<size>.csv`.
- If p99 is over 50 ms, write it in the results table and go on to the next size. If it is 50 ms or under, also write it in the results table and go on.
- When I tried this last week the 256 MB run took 14 minutes and the run ID was r-7f3a.
