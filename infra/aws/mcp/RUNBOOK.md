# AWS MCP benchmark runbook

Agent-driven provisioning and teardown for `photon-leptos-bench` campaigns. Uses **plugin-aws-core-aws-mcp** (not Terraform) as the primary path.

## Conventions

| Tag | Value |
|-----|-------|
| `Project` | `photon-leptos-bench` |
| `Campaign` | UUID per run |
| `Role` | `server` \| `loadgen` \| `alb` |

Staging bucket layout:

- `s3://<bucket>/campaigns/<uuid>/release.tar.gz`
- `s3://<bucket>/campaigns/<uuid>/reports/*.json`

## Phase 1 hardware profiles

Seven wired profiles in [`profiles.json`](profiles.json): `aws-t3-small`, `aws-t3-medium`, `aws-t4g-small`, `aws-t4g-medium`, `aws-t4g-large`, `aws-c7i-large`, `aws-c7i-xlarge`.

Load generators always use **`aws-c7i-large`** (`loadgen_profile` in profiles.json) — not swept as part of the matrix.

## Preflight

1. `aws___get_regional_availability` — confirm all Phase 1 instance types in target region.
2. `aws___call_aws` — `ec2 describe-vpcs`, `describe-subnets`; pick same-AZ subnets.
3. Pin AMIs in `profiles.json` via `ec2 describe-images` (replace `ami-PLACEHOLDER-*`).
   Those placeholders are intentional until preflight; do not launch campaigns against them.
4. Build or locate `photon-bench` from the photon core tree and export
   `PHOTON_BENCH_BIN` if it is not at `../photon/target/release/photon-bench`
   relative to this repo.

## Provision fleet (single campaign)

Use `aws___run_script` to:

1. Create security group: bench port **8080** from load-gen SG only; SSH/SSM as needed.
2. Launch:
   - 1× server at `--hardware` instance type (DUT)
   - 2× `c7i.large` load generators
3. Tag all resources with `Campaign=<uuid>`.

### BM-PLS9 (ALB horizontal smoke)

Additional MCP steps via `aws___call_aws`:

```text
elbv2 create-load-balancer
elbv2 create-target-group   # stickiness enabled for WS
elbv2 create-listener
elbv2 register-targets      # 2–4 server instances
```

Run benchmark with multiple server URLs:

```bash
photon-leptos-bench run --experiment bm-pls9 \
  --server-urls http://instance-a:8080,http://instance-b:8080 \
  --connections 256 --hardware aws-t3-medium
```

## Stage + bootstrap

1. Build release locally:

```bash
cargo build --release -p photon-leptos-bench
tar -czf release.tar.gz \
  target/release/photon-leptos-bench \
  target/release/photon-leptos-bench-server \
  infra/aws/mcp/user-data/
```

2. `aws___get_presigned_url` → upload tarball to staging bucket.
3. `aws___call_aws` `ssm send-command` on each instance with [`user-data/bootstrap.sh`](user-data/bootstrap.sh):

```bash
RELEASE_S3_URI=s3://bucket/campaigns/<uuid>/release.tar.gz bash bootstrap.sh
```

## Execute campaign slices

| Slice | Command |
|-------|---------|
| Substrate pairing | Run `photon-bench bm-p2` + `bm-pl2` on server EC2 first |
| `pls-connection` | `photon-leptos-bench matrix --slice pls-connection` |
| `pls-hub` | `photon-leptos-bench matrix --slice pls-hub --ws-mode per_subscribe` (includes hub A/B + PLS5) |
| `pls-client` | `photon-leptos-bench matrix --slice pls-client` |
| `pls-shape` | `photon-leptos-bench matrix --slice pls-shape` |
| `pls-soak` | `photon-leptos-bench matrix --slice pls-soak` (use `--duration-secs 3600` on AWS) |
| `pls-fleet` | `photon-leptos-bench matrix --slice pls-fleet` |
| `pls-hardware` | Full 7-profile matrix (PLS0 + PLS1 per profile) |

Load-gen invocation via [`user-data/run-matrix.sh`](user-data/run-matrix.sh):

```bash
SERVER_URL=http://<server-private-ip>:8080 bash run-matrix.sh pls-connection aws-t3-medium
```

## Collect results

1. `aws___run_script` — CloudWatch `CPUUtilization`, `NetworkIn/Out` for campaign window → merge into report JSON as `resource_profile`.
2. `aws s3 sync` reports from load gens to staging bucket.
3. Local: `photon-leptos-bench` reports under `photon-leptos-bench/reports/`.

## Teardown (always)

`aws___run_script` — terminate instances, delete ALB/SG/target groups by `Campaign=<uuid>` tag.

## Local smoke (before AWS)

```bash
# terminal 1
BENCH_ADDR=127.0.0.1:8080 cargo run -p photon-leptos-bench --bin photon-leptos-bench-server

# terminal 2
cargo run -p photon-leptos-bench -- run --experiment bm-pls0 \
  --server-url http://127.0.0.1:8080 --hardware dev-wsl --duration-secs 10 \
  --report photon-leptos-bench/reports/smoke-pls0.json
```
