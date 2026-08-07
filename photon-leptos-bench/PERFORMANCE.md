# photon-leptos performance

Measured on AWS instance profiles spanning burstable and compute-optimized shapes (`t3` / `t4g` / `c7i` classes). photon-leptos serves realtime UI/session traffic over Leptos/Axum with Photon underneath. Campaign notes come from AWS MCP bench runs.

## Serving capacity

Authoritative Campaign C on `t3.medium`: per_subscribe connection knee **256** sockets; broadcast_hub knee **768** (shared broadcast scope). Publish rate (PLS1) reaches **10,000** ops/s at N=64 before the connection knee binds.

Single-server connection and message rates scale with vCPU class; load generators should be at least as large as the device under test so the server remains the bottleneck. Horizontal serving behind a sticky load balancer is the shape for multi-instance fan-out. Older “~512 sockets/server” and “~200 hosts for 100k” projections are withdrawn; size from the Campaign C knees above. Horizontal / `c7i` remount coverage is still thin.

## Guidance

Pick instance class from AWS-tagged runs that match your connection concurrency and payload size. ARM (`t4g`) and x86 (`t3`/`c7i`) curves are not interchangeable without a fresh measure.

## How to read these results

AWS profile labels are authoritative. Developer-machine numbers are harness checks only.
