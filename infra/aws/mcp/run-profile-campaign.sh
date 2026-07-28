#!/usr/bin/env bash
# Run one DUT profile campaign (load gens must already exist).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/campaign-ssh.sh"

PROFILE="${1:?profile}"
INSTANCE_TYPE="${2:?instance_type}"
AMI="${3:?ami}"
LG_HOST="${4:?loadgen_public_ip}"
SUBNET="${SUBNET:?set SUBNET to your bench subnet id}"
SERVER_SG="${SERVER_SG:?set SERVER_SG to your bench security group id}"
KEY="${KEY:-photon-leptos-bench}"
DURATION="${DURATION:-60}"
CAMPAIGN="${CAMPAIGN:-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f}"

echo "=== Campaign profile: $PROFILE ($INSTANCE_TYPE) ==="
SERVER_ID=$(aws ec2 run-instances --region us-west-2 \
  --image-id "$AMI" --instance-type "$INSTANCE_TYPE" --key-name "$KEY" \
  --subnet-id "$SUBNET" --security-group-ids "$SERVER_SG" \
  --tag-specifications "ResourceType=instance,Tags=[{Key=Project,Value=photon-leptos-bench},{Key=Campaign,Value=$CAMPAIGN},{Key=Role,Value=server},{Key=Profile,Value=$PROFILE},{Key=Name,Value=plbench-$PROFILE}]" \
  --query 'Instances[0].InstanceId' --output text)

echo "Launched DUT $SERVER_ID"
aws ec2 wait instance-running --region us-west-2 --instance-ids "$SERVER_ID"
SERVER_IP=$(aws ec2 describe-instances --region us-west-2 --instance-ids "$SERVER_ID" \
  --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)
SERVER_PRIVATE=$(aws ec2 describe-instances --region us-west-2 --instance-ids "$SERVER_ID" \
  --query 'Reservations[0].Instances[0].PrivateIpAddress' --output text)
echo "DUT $SERVER_ID public=$SERVER_IP private=$SERVER_PRIVATE"

wait_ssh "$SERVER_IP"
scp_tar "$SERVER_IP"
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$REPO_ROOT/infra/aws/mcp/profiles.json" "${SSH_USER}@${SERVER_IP}:/tmp/profiles.json"
remote_bootstrap_server "$SERVER_IP"
ssh_cmd "$SERVER_IP" "sudo mv /tmp/profiles.json $INSTALL_DIR/profiles.json"
# Optional substrate helper from the photon core checkout (not this repo).
# Override with PHOTON_BENCH_BIN=/path/to/photon-bench when the sibling tree is elsewhere.
PHOTON_BENCH_BIN="${PHOTON_BENCH_BIN:-$REPO_ROOT/../photon/target/release/photon-bench}"
if [[ ! -x "$PHOTON_BENCH_BIN" ]]; then
  echo "error: photon-bench not found at $PHOTON_BENCH_BIN (set PHOTON_BENCH_BIN)" >&2
  exit 1
fi
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$PHOTON_BENCH_BIN" "${SSH_USER}@${SERVER_IP}:/tmp/photon-bench"
ssh_cmd "$SERVER_IP" "sudo mv /tmp/photon-bench $INSTALL_DIR/photon-bench && sudo chmod +x $INSTALL_DIR/photon-bench"
run_substrate "$SERVER_IP" "$PROFILE"
run_pls_connection_campaign "$LG_HOST" "$SERVER_IP" "$SERVER_PRIVATE" "$PROFILE" "$DURATION"

LOCAL_DEST="$REPO_ROOT/photon-leptos-bench/reports/$PROFILE"
mkdir -p "$LOCAL_DEST"
for f in bm-pls0-mem-embedded-${PROFILE}.json bm-pls1-mem-embedded-${PROFILE}.json \
         bm-pls0-rate100-${PROFILE}.json bm-pls0-rate1000-${PROFILE}.json \
         substrate-bm-p2-${PROFILE}.json substrate-bm-pl2-${PROFILE}.json; do
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no \
    "${SSH_USER}@${LG_HOST}:$INSTALL_DIR/reports/$f" "$LOCAL_DEST/" 2>/dev/null || \
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no \
    "${SSH_USER}@${SERVER_IP}:$INSTALL_DIR/reports/$f" "$LOCAL_DEST/" 2>/dev/null || true
done
cp "$LOCAL_DEST"/bm-pls*.json "$REPO_ROOT/photon-leptos-bench/reports/" 2>/dev/null || true

aws ec2 terminate-instances --region us-west-2 --instance-ids "$SERVER_ID" --output text >/dev/null
echo "Terminated DUT $SERVER_ID"
echo "=== Done $PROFILE ==="
